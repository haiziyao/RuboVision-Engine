use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuncParam{
    pub function_id: String,
    pub args: Vec<String>,
}
 

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct FuncParamConfig{
    pub func_param_list: Vec<FuncParam>,
}

 