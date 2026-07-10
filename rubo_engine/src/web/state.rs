use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock as StdRwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use tokio::sync::{RwLock, mpsc, oneshot};

use crate::{
    Message,
    config::{AppConfig, RuboConfig},
    web::{WebConfig, WebHistory, WebHub},
};

#[derive(Clone)]
pub struct WebState {
    config: WebConfig,
    app_config: AppConfig,
    rubo_config: Arc<StdRwLock<RuboConfig>>,
    runtime_config: Arc<StdRwLock<RuboConfig>>,
    history: Arc<RwLock<WebHistory>>,
    hub: WebHub,
    root: PathBuf,
    source_sender: Arc<StdRwLock<Option<mpsc::Sender<Message>>>>,
    runtime_control: Arc<StdRwLock<Option<WebRuntimeControl>>>,
}

impl WebState {
    pub fn new(root: impl AsRef<Path>, app_config: AppConfig, rubo_config: RuboConfig) -> Self {
        let config = WebConfig::from_app_web(app_config.web());
        Self::with_config(root, app_config, rubo_config, config)
    }

    pub fn with_config(
        root: impl AsRef<Path>,
        app_config: AppConfig,
        rubo_config: RuboConfig,
        config: WebConfig,
    ) -> Self {
        Self::with_shared_config(
            root,
            app_config,
            Arc::new(StdRwLock::new(rubo_config)),
            config,
        )
    }

    pub fn with_shared_config(
        root: impl AsRef<Path>,
        app_config: AppConfig,
        rubo_config: Arc<StdRwLock<RuboConfig>>,
        config: WebConfig,
    ) -> Self {
        let runtime_config = Arc::new(StdRwLock::new(
            rubo_config
                .read()
                .expect("web config lock poisoned")
                .clone(),
        ));
        Self {
            history: Arc::new(RwLock::new(WebHistory::new(config.history_limit()))),
            hub: WebHub::new(config.history_limit()),
            root: root.as_ref().to_path_buf(),
            config,
            app_config,
            rubo_config,
            runtime_config,
            source_sender: Arc::new(StdRwLock::new(None)),
            runtime_control: Arc::new(StdRwLock::new(None)),
        }
    }

    pub fn config(&self) -> &WebConfig {
        &self.config
    }

    pub fn app_config(&self) -> &AppConfig {
        &self.app_config
    }

    pub fn rubo_config(&self) -> Arc<StdRwLock<RuboConfig>> {
        self.rubo_config.clone()
    }

    pub fn runtime_config(&self) -> Arc<StdRwLock<RuboConfig>> {
        self.runtime_config.clone()
    }

    pub fn set_runtime_config(&self, config: RuboConfig) {
        *self
            .runtime_config
            .write()
            .expect("web runtime config lock poisoned") = config;
    }

    pub fn history(&self) -> Arc<RwLock<WebHistory>> {
        self.history.clone()
    }

    pub fn hub(&self) -> &WebHub {
        &self.hub
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source_sender(&self) -> Option<mpsc::Sender<Message>> {
        self.source_sender
            .read()
            .expect("web source sender lock poisoned")
            .clone()
    }

    pub fn set_source_sender(&mut self, sender: mpsc::Sender<Message>) {
        *self
            .source_sender
            .write()
            .expect("web source sender lock poisoned") = Some(sender);
    }

    pub fn runtime_control(&self) -> Option<WebRuntimeControl> {
        self.runtime_control
            .read()
            .expect("web runtime control lock poisoned")
            .clone()
    }

    pub fn set_runtime_control(&mut self, control: WebRuntimeControl) {
        *self
            .runtime_control
            .write()
            .expect("web runtime control lock poisoned") = Some(control);
    }
}

#[derive(Clone)]
pub struct WebRuntimeControl {
    sender: mpsc::Sender<WebRuntimeCommand>,
    running: Arc<AtomicBool>,
}

impl WebRuntimeControl {
    pub fn new(sender: mpsc::Sender<WebRuntimeCommand>, running: Arc<AtomicBool>) -> Self {
        Self { sender, running }
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub async fn start(&self) -> Result<(), String> {
        self.send_command(WebRuntimeCommandKind::Start).await
    }

    pub async fn stop(&self) -> Result<(), String> {
        self.send_command(WebRuntimeCommandKind::Stop).await
    }

    pub async fn restart(&self) -> Result<(), String> {
        self.send_command(WebRuntimeCommandKind::Restart).await
    }

    async fn send_command(&self, kind: WebRuntimeCommandKind) -> Result<(), String> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(WebRuntimeCommand {
                kind,
                result: sender,
            })
            .await
            .map_err(|error| error.to_string())?;
        receiver.await.map_err(|error| error.to_string())?
    }
}

pub struct WebRuntimeCommand {
    pub kind: WebRuntimeCommandKind,
    pub result: oneshot::Sender<Result<(), String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebRuntimeCommandKind {
    Start,
    Stop,
    Restart,
}
