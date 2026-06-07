use std::collections::HashMap;

use anyhow::{Result, anyhow};

use crate::config::FunctionsConfig;
use crate::func::{FUNCTION_DESCRIPTORS, FuncWorkerMap, FunctionDescriptor};

pub fn register_func(cfg: FunctionsConfig) -> Result<FuncWorkerMap> {
    let descriptors: HashMap<&str, &FunctionDescriptor> = FUNCTION_DESCRIPTORS
        .iter()
        .map(|descriptor| (descriptor.id, descriptor))
        .collect();
    let mut map = FuncWorkerMap::new();

    for entry in cfg.entries {
        let descriptor = descriptors
            .get(entry.function_id.as_str())
            .ok_or_else(|| anyhow!("unknown function_id `{}`", entry.function_id))?;
        let function = (descriptor.build)(&entry)?;
        map.add(&entry.function_id, function);
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use crate::config::load_config;
    use crate::func::FUNCTION_DESCRIPTORS;

    use super::register_func;

    #[test]
    fn declarative_registry_contains_builtin_functions() {
        let ids: Vec<&str> = FUNCTION_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.id)
            .collect();

        assert_eq!(
            ids,
            vec!["color_detect", "qr_detect", "cross_detect", "debug_fun"]
        );
    }

    #[test]
    fn register_func_rejects_unknown_function() {
        let mut functions = load_config().expect("load config").functions;
        functions.entries[0].function_id = "missing_function".to_string();

        assert!(register_func(functions).is_err());
    }

    #[test]
    fn register_func_rejects_invalid_typed_params() {
        let mut functions = load_config().expect("load config").functions;
        let color = functions
            .entries
            .iter_mut()
            .find(|entry| entry.function_id == "color_detect")
            .expect("color config");
        color
            .params
            .as_table_mut()
            .expect("color params table")
            .remove("loop_count");

        assert!(register_func(functions).is_err());
    }

    #[test]
    fn register_func_rejects_out_of_range_typed_params() {
        let mut functions = load_config().expect("load config").functions;
        let color = functions
            .entries
            .iter_mut()
            .find(|entry| entry.function_id == "color_detect")
            .expect("color config");
        color
            .params
            .as_table_mut()
            .expect("color params table")
            .insert("radius_ratio".to_string(), toml::Value::Float(2.0));

        assert!(register_func(functions).is_err());
    }
}
