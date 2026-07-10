#[path = "traits/binding.rs"]
pub mod binding;
#[path = "traits/config.rs"]
pub mod config;
#[path = "traits/device.rs"]
pub mod device;
#[path = "traits/func.rs"]
pub mod func;
#[path = "traits/sink.rs"]
pub mod sink;
#[path = "traits/source.rs"]
pub mod source;

pub use binding::{BindingConfig, BindingSourceConfig};
pub use config::ConfigAccess;
pub use device::DeviceConfig;
pub use func::FuncConfig;
pub use sink::SinkConfig;
pub use source::SourceConfig;
