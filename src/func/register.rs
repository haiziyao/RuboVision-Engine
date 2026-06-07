use crate::config::{FunctionsConfig, ReturnTargets};
use crate::func::FuncWorkerMap;
use crate::func::tarits::*;
use crate::func::usual::*;
use anyhow::{Context, Result};

pub fn register_func(cfg: FunctionsConfig) -> Result<FuncWorkerMap> {
    let mut map = FuncWorkerMap::new();
    for entry in cfg.entries {
        let args = entry
            .legacy_args()
            .with_context(|| format!("invalid params for `{}`", entry.function_id))?;
        map.add(
            &entry.function_id,
            function_factory(&entry.function_id, &entry.returns, &args),
        );
    }
    Ok(map)
}

fn function_factory(function_id: &str, returns: &ReturnTargets, args: &[String]) -> FunctionDef {
    match function_id {
        "debug_fun" => FunctionDef::new(function_id, args.to_owned(), returns.clone(), fn_debug),
        "color_detect" => FunctionDef::new(
            function_id,
            args.to_owned(),
            returns.clone(),
            fn_color_detect,
        ),
        "qr_detect" => {
            FunctionDef::new(function_id, args.to_owned(), returns.clone(), fn_qr_detect)
        }
        "cross_detect" => FunctionDef::new(
            function_id,
            args.to_owned(),
            returns.clone(),
            fn_cross_detect,
        ),
        _ => panic!("unknown function_id `{function_id}`"),
    }
}
