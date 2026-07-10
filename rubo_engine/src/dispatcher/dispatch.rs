use crate::{
    DispatchError, DispatchErrorKind,
    config::{BindingConfig, RuboConfig},
    log::{text, warn_text},
};
use tracing::{info, info_span};

use super::{DispatchMessage, DispatchOutput, TaskRequest};

pub fn dispatch(config: &RuboConfig, message: DispatchMessage) -> DispatchOutput {
    let (source_id, message) = message.into_parts();
    let key = message.key().to_string();
    let _span = info_span!("dispatcher.dispatch", source_id = %source_id, key = %key).entered();
    info!(
        "{}",
        text(format!(
            "dispatcher.dispatch.start source={source_id} key={key}"
        ))
    );

    let mut matched = config
        .bindings()
        .values()
        .filter(|binding| binding_matches(binding, &source_id, &key));
    let first = matched.next();
    let second = matched.next();

    match (first, second) {
        (None, _) => {
            info!(
                "{}",
                warn_text(format!(
                    "dispatcher.dispatch.error source={} key={} kind=BindingNotFound",
                    source_id, key
                ))
            );
            DispatchOutput::Error(DispatchError::new(
                source_id,
                key,
                message,
                DispatchErrorKind::BindingNotFound,
            ))
        }
        (Some(_), Some(_)) => {
            info!(
                "{}",
                warn_text(format!(
                    "dispatcher.dispatch.error source={} key={} kind=BindingConflict",
                    source_id, key
                ))
            );
            DispatchOutput::Error(DispatchError::new(
                source_id,
                key,
                message,
                DispatchErrorKind::BindingConflict,
            ))
        }
        (Some(binding), None) => {
            if binding_config_invalid(binding) {
                info!(
                    "{}",
                    warn_text(format!(
                        "dispatcher.dispatch.error source={} key={} binding={} kind=ConfigInvalid",
                        source_id,
                        key,
                        binding.id()
                    ))
                );
                return DispatchOutput::Error(DispatchError::new(
                    source_id,
                    key,
                    message,
                    DispatchErrorKind::ConfigInvalid,
                ));
            }
            info!(
                "{}",
                text(format!(
                    "dispatcher.dispatch.task binding={} func={} devices={} sinks={}",
                    binding.id(),
                    binding.func_ref(),
                    binding.devices().len(),
                    binding.sinks().len()
                ))
            );
            DispatchOutput::Task(TaskRequest::new(
                binding.id(),
                source_id,
                key,
                binding.func_ref(),
                message,
                binding.devices().to_vec(),
                binding.sinks().to_vec(),
            ))
        }
    }
}

fn binding_matches(binding: &BindingConfig, source_id: &str, key: &str) -> bool {
    binding.source_ref().id() == source_id && binding.source_ref().event() == key
}

fn binding_config_invalid(binding: &BindingConfig) -> bool {
    binding.id().is_empty()
        || binding.source_ref().id().is_empty()
        || binding.source_ref().event().is_empty()
        || binding.func_ref().is_empty()
}
