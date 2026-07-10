use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
        atomic::AtomicBool,
        atomic::{AtomicU8, Ordering},
    },
};

use tokio::{
    sync::{Mutex as TokioMutex, mpsc},
    task::JoinHandle,
};

use crate::{
    ChannelSinkFactory, ChannelSourceFactory, Device, DeviceRegister, Function, FunctionRegister,
    IntervalSourceFactory, ManualSourceFactory, Message, Output, RuntimeError, RuntimeResources,
    Sink, SinkError, SinkRegister, SourceRegister, WebConfig, WebRuntimeCommand,
    WebRuntimeCommandKind, WebRuntimeControl, WebSink, WebState,
    config::{AppConfig, BindingConfig, RuboConfig, SinkConfig, SourceConfig},
    register_inventory, run_config_with_resources,
    web::{WEB_SINK_ID, WEB_SOURCE_ID, WebError, serve},
};

const DEFAULT_WEB_CHANNEL_SIZE: usize = 1024;
const RUNTIME_RUNNING: u8 = 0;
const RUNTIME_STOPPED: u8 = 1;
const RUNTIME_FINISHED: u8 = 2;

pub struct Engine {
    root: PathBuf,
    app_config: AppConfig,
    config: Arc<RwLock<RuboConfig>>,
    source_register: SourceRegister,
    device_register: DeviceRegister,
    functions: FunctionRegister,
    sinks: SinkRegister,
    resources: RuntimeResources,
    web_state: Option<WebState>,
}

impl Engine {
    pub fn new(root: impl AsRef<Path>, app_config: AppConfig, config: RuboConfig) -> Self {
        let mut source_register = SourceRegister::new();
        source_register.register("channel", ChannelSourceFactory);
        source_register.register("manual", ManualSourceFactory);
        source_register.register("interval", IntervalSourceFactory);

        let mut sinks = SinkRegister::new();
        sinks.register_factory("channel", ChannelSinkFactory);

        let mut device_register = DeviceRegister::new();
        let mut functions = FunctionRegister::new();
        register_inventory(
            &mut source_register,
            &mut device_register,
            &mut functions,
            &mut sinks,
        );

        Self {
            root: root.as_ref().to_path_buf(),
            app_config,
            config: Arc::new(RwLock::new(config)),
            source_register,
            device_register,
            functions,
            sinks,
            resources: RuntimeResources::new(),
            web_state: None,
        }
    }

    pub fn config(&self) -> RwLockReadGuard<'_, RuboConfig> {
        self.config.read().expect("engine config lock poisoned")
    }

    pub fn config_mut(&mut self) -> RwLockWriteGuard<'_, RuboConfig> {
        self.config.write().expect("engine config lock poisoned")
    }

    pub fn sinks(&self) -> &SinkRegister {
        &self.sinks
    }

    pub fn web_state(&self) -> Option<&WebState> {
        self.web_state.as_ref()
    }

    pub fn source_register_mut(&mut self) -> &mut SourceRegister {
        &mut self.source_register
    }

    pub fn sink_register_mut(&mut self) -> &mut SinkRegister {
        &mut self.sinks
    }

    pub fn register_device<T>(&mut self, kind: impl Into<String>)
    where
        T: Device,
    {
        self.device_register.register_device::<T>(kind);
    }

    pub fn register_function<T>(&mut self, id: impl Into<String>, function: T)
    where
        T: Function,
    {
        self.functions.register(id, function);
    }

    pub fn register_sink<T>(&mut self, id: impl Into<String>, sink: T)
    where
        T: Sink,
    {
        self.sinks.register(id, sink);
    }

    pub fn insert_source_channel(
        &mut self,
        id: impl Into<String>,
        receiver: mpsc::Receiver<Message>,
    ) {
        self.resources.insert_source_channel(id, receiver);
    }

    pub fn insert_sink_channel(&mut self, id: impl Into<String>, sender: mpsc::Sender<Output>) {
        self.resources.insert_sink_channel(id, sender);
    }

    pub fn prepare_web(&mut self) {
        if !self.app_config.web().enabled() || self.web_state.is_some() {
            return;
        }

        let mut runtime_config = self
            .config
            .read()
            .expect("engine config lock poisoned")
            .clone();
        Self::ensure_web_config(&mut runtime_config);
        Self::derive_debug_bindings(&mut runtime_config);

        let web_config = WebConfig::from_app_web(self.app_config.web());
        let mut web_state = WebState::with_shared_config(
            &self.root,
            self.app_config.clone(),
            self.config.clone(),
            web_config,
        );
        web_state.set_runtime_config(runtime_config);
        let (sender, receiver) = mpsc::channel(DEFAULT_WEB_CHANNEL_SIZE);
        web_state.set_source_sender(sender);
        self.resources
            .insert_source_channel(WEB_SOURCE_ID, receiver);
        self.sinks.register(
            WEB_SINK_ID,
            WebSink::new(web_state.history(), web_state.hub().clone()),
        );
        self.web_state = Some(web_state);
    }

    pub async fn run(
        &mut self,
        channel_size: usize,
    ) -> Result<Vec<crate::SourceRunOutput>, RuntimeError> {
        self.refresh_web_source_channel();
        let mut config = self
            .config
            .read()
            .expect("engine config lock poisoned")
            .clone();
        if let Some(web_state) = self.web_state.as_ref() {
            Self::ensure_web_config(&mut config);
            Self::derive_debug_bindings(&mut config);
            web_state.set_runtime_config(config.clone());
        }
        self.register_missing_config_sinks(&config)
            .map_err(|error| RuntimeError::ConfigInvalid {
                message: error.to_string(),
            })?;
        run_config_with_resources(
            &config,
            &self.source_register,
            &mut self.resources,
            &self.device_register,
            &self.functions,
            &self.sinks,
            channel_size,
        )
        .await
    }

    pub async fn run_once(
        &mut self,
        channel_size: usize,
    ) -> Result<Vec<crate::SourceRunOutput>, RuntimeError> {
        self.run(channel_size).await
    }

    pub fn start(self, channel_size: usize) -> EngineRuntimeHandle {
        EngineRuntimeHandle::spawn_owned(self, channel_size)
    }

    pub fn runtime(self, channel_size: usize) -> EngineRuntime {
        EngineRuntime::new(self, channel_size)
    }

    pub async fn serve_web(&mut self) -> Result<(), WebError> {
        self.prepare_web();
        let Some(state) = self.web_state.clone() else {
            return Err(WebError::runtime("web is disabled"));
        };
        serve(state).await
    }

    fn register_missing_config_sinks(&mut self, config: &RuboConfig) -> Result<(), SinkError> {
        for sink_config in config.sinks().values() {
            if self.sinks.contains(sink_config.id()) {
                continue;
            }
            self.sinks
                .register_config_sink_with_resources(sink_config, &mut self.resources)?;
        }
        Ok(())
    }

    fn refresh_web_source_channel(&mut self) {
        let Some(web_state) = self.web_state.as_mut() else {
            return;
        };
        let (sender, receiver) = mpsc::channel(DEFAULT_WEB_CHANNEL_SIZE);
        web_state.set_source_sender(sender);
        self.resources
            .insert_source_channel(WEB_SOURCE_ID, receiver);
    }

    fn ensure_web_config(config: &mut RuboConfig) {
        config
            .sources_mut()
            .entry(WEB_SOURCE_ID.to_string())
            .or_insert_with(|| SourceConfig::new(WEB_SOURCE_ID).kind("channel"));
        config
            .sinks_mut()
            .entry(WEB_SINK_ID.to_string())
            .or_insert_with(|| SinkConfig::new(WEB_SINK_ID).kind("web"));
    }

    fn derive_debug_bindings(config: &mut RuboConfig) {
        let bindings: Vec<_> = config
            .bindings()
            .values()
            .filter(|binding| binding.debug_enabled())
            .cloned()
            .collect();

        for binding in bindings {
            if binding.source_ref().id() == WEB_SOURCE_ID
                && binding.source_ref().event() == binding.id()
            {
                continue;
            }
            let id = format!("web.debug.{}", binding.id());
            if config.bindings().contains_key(&id) {
                continue;
            }
            let mut debug_binding = BindingConfig::new(&id)
                .source(WEB_SOURCE_ID, binding.id())
                .func(binding.func_ref())
                .debug(false);
            for device in binding.devices() {
                debug_binding = debug_binding.device(device);
            }
            for sink in binding.sinks() {
                debug_binding = debug_binding.sink(sink);
            }
            if !debug_binding.sinks().iter().any(|sink| sink == WEB_SINK_ID) {
                debug_binding = debug_binding.sink(WEB_SINK_ID);
            }
            config.bindings_mut().insert(id, debug_binding);
        }
    }
}

pub struct EngineRuntime {
    engine: Arc<TokioMutex<Engine>>,
    inner: Arc<TokioMutex<EngineRuntimeInner>>,
}

impl EngineRuntime {
    pub fn new(mut engine: Engine, channel_size: usize) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(32);
        let running = Arc::new(AtomicBool::new(false));
        let control = WebRuntimeControl::new(command_sender, running.clone());
        if let Some(web_state) = engine.web_state.as_mut() {
            web_state.set_runtime_control(control);
        }

        let engine = Arc::new(TokioMutex::new(engine));
        let inner = Arc::new(TokioMutex::new(EngineRuntimeInner {
            engine: engine.clone(),
            channel_size,
            handle: None,
            running: running.clone(),
        }));
        Self::spawn_command_loop(inner.clone(), command_receiver);
        Self { engine, inner }
    }

    pub fn engine(&self) -> Arc<TokioMutex<Engine>> {
        self.engine.clone()
    }

    pub fn is_running(&self) -> bool {
        self.inner.try_lock().is_ok_and(|inner| inner.is_running())
    }

    pub fn start(&mut self) {
        if let Ok(mut inner) = self.inner.try_lock() {
            inner.start();
        }
    }

    pub async fn stop(&mut self) {
        self.inner.lock().await.stop().await;
    }

    pub async fn restart(&mut self) {
        self.inner.lock().await.restart().await;
    }

    fn spawn_command_loop(
        inner: Arc<TokioMutex<EngineRuntimeInner>>,
        mut receiver: mpsc::Receiver<WebRuntimeCommand>,
    ) {
        tokio::spawn(async move {
            while let Some(command) = receiver.recv().await {
                let result = match command.kind {
                    WebRuntimeCommandKind::Start => {
                        inner.lock().await.start();
                        Ok(())
                    }
                    WebRuntimeCommandKind::Stop => {
                        inner.lock().await.stop().await;
                        Ok(())
                    }
                    WebRuntimeCommandKind::Restart => {
                        inner.lock().await.restart().await;
                        Ok(())
                    }
                };
                let _ = command.result.send(result);
            }
        });
    }
}

struct EngineRuntimeInner {
    engine: Arc<TokioMutex<Engine>>,
    channel_size: usize,
    handle: Option<EngineRuntimeHandle>,
    running: Arc<AtomicBool>,
}

impl EngineRuntimeInner {
    fn is_running(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(EngineRuntimeHandle::is_running)
    }

    fn start(&mut self) {
        if self.is_running() {
            return;
        }
        self.running.store(true, Ordering::SeqCst);
        self.handle = Some(EngineRuntimeHandle::spawn_shared(
            self.engine.clone(),
            self.channel_size,
            Some(self.running.clone()),
        ));
    }

    async fn stop(&mut self) {
        let Some(handle) = self.handle.take() else {
            self.running.store(false, Ordering::SeqCst);
            return;
        };
        self.running.store(false, Ordering::SeqCst);
        handle.state.store(RUNTIME_STOPPED, Ordering::SeqCst);
        handle.task.abort();
        let _ = handle.task.await;
    }

    async fn restart(&mut self) {
        self.stop().await;
        self.start();
    }
}

pub struct EngineRuntimeHandle {
    state: Arc<AtomicU8>,
    task: JoinHandle<Result<Vec<crate::SourceRunOutput>, RuntimeError>>,
}

impl EngineRuntimeHandle {
    fn spawn_owned(engine: Engine, channel_size: usize) -> Self {
        let state = Arc::new(AtomicU8::new(RUNTIME_RUNNING));
        let task_state = state.clone();
        let task = tokio::spawn(async move {
            let mut engine = engine;
            let result = engine.run(channel_size).await;
            if task_state.load(Ordering::SeqCst) == RUNTIME_RUNNING {
                task_state.store(RUNTIME_FINISHED, Ordering::SeqCst);
            }
            result
        });
        Self { state, task }
    }

    fn spawn_shared(
        engine: Arc<TokioMutex<Engine>>,
        channel_size: usize,
        running: Option<Arc<AtomicBool>>,
    ) -> Self {
        let state = Arc::new(AtomicU8::new(RUNTIME_RUNNING));
        let task_state = state.clone();
        let task = tokio::spawn(async move {
            let mut engine = engine.lock().await;
            let result = engine.run(channel_size).await;
            if let Some(running) = running {
                running.store(false, Ordering::SeqCst);
            }
            if task_state.load(Ordering::SeqCst) == RUNTIME_RUNNING {
                task_state.store(RUNTIME_FINISHED, Ordering::SeqCst);
            }
            result
        });
        Self { state, task }
    }

    pub fn is_running(&self) -> bool {
        self.state.load(Ordering::SeqCst) == RUNTIME_RUNNING && !self.task.is_finished()
    }

    pub fn is_stopped(&self) -> bool {
        self.state.load(Ordering::SeqCst) == RUNTIME_STOPPED
    }

    pub fn is_finished(&self) -> bool {
        self.state.load(Ordering::SeqCst) == RUNTIME_FINISHED
    }

    pub async fn stop(&self) {
        self.state.store(RUNTIME_STOPPED, Ordering::SeqCst);
        self.task.abort();
    }

    pub async fn wait(self) -> Result<Vec<crate::SourceRunOutput>, RuntimeError> {
        match self.task.await {
            Ok(result) => result,
            Err(error) => {
                if self.state.load(Ordering::SeqCst) != RUNTIME_STOPPED {
                    self.state.store(RUNTIME_FINISHED, Ordering::SeqCst);
                }
                Err(RuntimeError::TaskJoin {
                    message: error.to_string(),
                })
            }
        }
    }
}
