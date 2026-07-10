use async_trait::async_trait;
use rubo_engine::{FuncResult, Function, FunctionCall, FunctionError, config::ConfigAccess};
use serde_json::json;

#[rubo_engine::function(id = "debug_fun")]
#[derive(Default)]
pub struct DebugFun;

#[async_trait]
impl Function for DebugFun {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let message = call
            .function_config()
            .get_or("message", "debug".to_string())?;
        let result = format!("{message} success");
        Ok(FuncResult::new(json!({
            "text": result.clone(),
            "value": result,
            "image": "data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='640' height='360' viewBox='0 0 640 360'%3E%3Crect width='640' height='360' fill='%23263d39'/%3E%3Crect x='24' y='24' width='592' height='312' fill='%23ffffff'/%3E%3Ctext x='320' y='190' text-anchor='middle' font-family='Arial' font-size='38' fill='%231c765e'%3Edebug success%3C/text%3E%3C/svg%3E"
        })))
    }
}

#[cfg(test)]
mod tests {
    use rubo_engine::{FunctionDevices, Message};

    use super::*;
    use crate::default_rubo_config;

    #[tokio::test]
    async fn debug_fun_test() {
        let config = default_rubo_config();
        let function_config = &config.funcs()["debug_fun"];
        let message = Message::new("debug");
        let call = FunctionCall::new(function_config, &message, FunctionDevices::new());

        let result = DebugFun.call(call).await.expect("debug function");

        assert_eq!(result.value()["text"], "debug success");
        assert_eq!(result.value()["value"], "debug success");
        assert!(
            result.value()["image"]
                .as_str()
                .is_some_and(|image| image.starts_with("data:image/"))
        );
    }
}
