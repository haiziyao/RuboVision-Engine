use std::path::Path;

use crate::config::RuntimeConfig;

pub fn load_config() -> Result<RuntimeConfig, config::ConfigError> {
    load_config_from("config")
}

fn load_config_from(base: impl AsRef<Path>) -> Result<RuntimeConfig, config::ConfigError> {
    let base = base.as_ref();
    let builder = config::Config::builder()
        .add_source(config::File::from(base.join("app.yaml")).required(true))
        .add_source(config::File::from(base.join("message.yaml")).required(true))
        .add_source(config::File::from(base.join("bindings.toml")).required(true))
        .add_source(config::File::from(base.join("functions.toml")).required(true))
        .add_source(config::File::from(base.join("device.toml")).required(true))
        .add_source(config::Environment::with_prefix("RUBO"));

    let cfg = builder.build()?;
    let runtime: RuntimeConfig = cfg.try_deserialize()?;
    runtime.validate()?;
    Ok(runtime)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{load_config, load_config_from};

    struct TempConfigDir(PathBuf);

    impl TempConfigDir {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("rubovision-config-{}-{stamp}", std::process::id()));
            fs::create_dir_all(&path).expect("create temp config directory");
            Self(path)
        }

        fn write(&self, name: &str, content: &str) {
            fs::write(self.0.join(name), content).expect("write temp config file");
        }
    }

    impl Drop for TempConfigDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn load_config_uses_typed_runtime_sections() {
        let cfg = load_config().expect("load typed runtime config");

        assert!(cfg.message.web.on);
        assert_eq!(cfg.message.uart.serial, "/dev/ttyV0");
        assert_eq!(cfg.bindings.uart_source[0].source_key, 0x01);
        assert_eq!(cfg.bindings.debug_source[0].source_key, "color");
        assert_eq!(cfg.devices.list[0].device_id, "color_camera");
        assert_eq!(cfg.devices.list[0].path, "/dev/video2");
        assert!(cfg.functions.entries.iter().any(|entry| {
            entry.function_id == "color_detect"
                && entry.returns.uart
                && entry.returns.gpio.as_deref() == Some("color")
        }));
    }

    #[test]
    fn load_config_rejects_removed_legacy_sections() {
        let dir = TempConfigDir::new();
        dir.write(
            "app.yaml",
            "app:\n  name: test\n  profile: test\n  log_level: info\n",
        );
        dir.write(
            "message.yaml",
            "message:\n  web: { on: false, host: 127.0.0.1, port: 3000 }\n  uart: { on: false, serial: /dev/null, baud: 9600, data_bit: 8, stop_bit: 1, parity_bit: false }\n  gpio: { on: false, active_low: true, run_pin: 27, signals: {} }\n",
        );
        dir.write(
            "bindings.toml",
            "[bindings]\nuart_source = []\ntimer_source = []\nloop_source = []\ndebug_source = []\n",
        );
        dir.write("device.toml", "[devices]\nlist = []\n");
        dir.write(
            "functions.toml",
            "[functions]\nentries = []\n[func_param_config]\nfunc_param_list = []\n",
        );

        let error = load_config_from(&dir.0).expect_err("legacy section must be rejected");
        assert!(error.to_string().contains("func_param_config"));
    }
}
