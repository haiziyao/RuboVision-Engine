mod device;

use anyhow::{Result};
use device::config::DeviceConfig;

use crate::device::config;

fn main() -> Result<()> {
    let my_config = getConfig()?; 
    device::camera::main(my_config.color_camera_config)?;
   Ok(())
}




fn getConfig() -> Result<DeviceConfig> {
    let filepath = "/config/param.toml";
    DeviceConfig::from_file(filepath)
} 
