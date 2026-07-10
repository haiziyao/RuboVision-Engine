use std::{collections::HashMap, sync::Arc};

use crate::Function;

#[derive(Clone, Default)]
pub struct FunctionRegister {
    funcs: HashMap<String, Arc<dyn Function>>,
}

impl FunctionRegister {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T>(&mut self, id: impl Into<String>, func: T)
    where
        T: Function,
    {
        self.funcs.insert(id.into(), Arc::new(func));
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Function>> {
        self.funcs.get(id).cloned()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.funcs.contains_key(id)
    }
}
