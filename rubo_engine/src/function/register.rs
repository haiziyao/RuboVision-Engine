use std::{collections::HashMap, sync::Arc};

use crate::{Function, FunctionAspect};

#[derive(Clone, Default)]
pub struct FunctionRegister {
    funcs: HashMap<String, Arc<dyn Function>>,
    aspects: Vec<Arc<dyn FunctionAspect>>,
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

    pub(crate) fn register_aspect<T>(&mut self, aspect: T)
    where
        T: FunctionAspect,
    {
        self.aspects.push(Arc::new(aspect));
    }

    pub(crate) fn aspects(&self) -> &[Arc<dyn FunctionAspect>] {
        &self.aspects
    }
}
