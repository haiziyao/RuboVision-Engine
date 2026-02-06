

fn getConfig() -> Result<DeviceConfig> {
    let filepath = "/config/param.toml";
    DeviceConfig::from_file(filepath)
} 