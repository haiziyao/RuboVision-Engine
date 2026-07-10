use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use crate::ConfigError;

pub trait ConfigAccess {
    fn values(&self) -> &Map<String, Value>;
    fn values_mut(&mut self) -> &mut Map<String, Value>;

    fn get<T>(&self, key: &str) -> Result<T, ConfigError>
    where
        T: DeserializeOwned,
    {
        let Some(value) = self.values().get(key) else {
            return Err(ConfigError::ConfigFormat {
                message: format!("missing config key `{key}`"),
            });
        };

        serde_json::from_value(value.clone()).map_err(|error| ConfigError::ConfigFormat {
            message: format!("invalid config key `{key}`: {error}"),
        })
    }

    fn get_or<T>(&self, key: &str, default: T) -> Result<T, ConfigError>
    where
        T: DeserializeOwned,
    {
        match self.values().get(key) {
            Some(value) => {
                serde_json::from_value(value.clone()).map_err(|error| ConfigError::ConfigFormat {
                    message: format!("invalid config key `{key}`: {error}"),
                })
            }
            None => Ok(default),
        }
    }

    fn contains(&self, key: &str) -> bool {
        self.values().contains_key(key)
    }

    fn raw(&self, key: &str) -> Option<&Value> {
        self.values().get(key)
    }

    fn set<T>(mut self, key: &str, value: T) -> Self
    where
        Self: Sized,
        T: Serialize,
    {
        self.values_mut().insert(
            key.to_string(),
            serde_json::to_value(value).expect("config value must serialize"),
        );
        self
    }
}
