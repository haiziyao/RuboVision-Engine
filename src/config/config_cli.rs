use clap::Parser;
use anyhow::{Context, Ok, Result};
use std::fs;
use std::path::Path;
use toml_edit::{value, DocumentMut};
use colored::*;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    
    #[arg(short, long)]
    key: Vec<String>,

    #[arg(short, long)]
    value: Vec<String>,
}




pub fn register_config_cli() -> Result<()>{
    let args = Args::parse();

    if args.key.len() != args.value.len() {
        println!("num(key) is not equal to num(value), setting failed");
        return Ok(())
    }

    let path = Path::new("config/param.toml");
    let s = fs::read_to_string(path).with_context(|| format!("filepath not found {:?}", path))?;

    let mut doc = s.parse::<DocumentMut>()
        .with_context(|| "TOML read failed")?;
    
    for (k, v) in args.key.iter().zip(args.value.iter()) {
        let (a, b) = key_parse(k).expect("unknown key");
        let item = &mut doc[a][b];
        let old = item.to_string();
        *item = value(v.as_str());
        println!("{}",format!("{k} has updated, from {old} to {v}").green());
    }
    let new_s = doc.to_string();
    atomic_write(path, &new_s)?;

    Ok(())
}

fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn key_parse(s: &str) -> Option<(&'static str, &'static str)> {
    match s {
        // camera
        "color_cam" => Some(("color_camera_config", "color_camera")),
        "qr_cam"    => Some(("qr_camera_config", "qr_camera")),
        "cross_cam" => Some(("cross_camera_config", "cross_camera")),

        // gpio
        "serial"    => Some(("gpio_config", "serial")),
        _ => None,
    }
}