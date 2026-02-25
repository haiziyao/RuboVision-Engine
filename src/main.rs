mod config;
mod device;
mod utils;

use crate::config::config_cli::register_config_cli;
use crate::{device::qr_detect::qr_detect_work, utils::device_config_util::get_config};
use crate::device::color_detect::color_detect_work;
use anyhow::{Result};
use std::sync::mpsc;
use std::thread;
use crate::device::gpio::gpio_work::{receive_line_loop,register_gpio,send_line,register_lights};
 
fn main() -> Result<()> {
    // 命令行给参
    let _ = register_config_cli()?;
    // 读取配置
    let my_config = get_config()?; 
    let (gpio_config,qr_config,color_config,light_config) = 
    (my_config.gpio_config,my_config.qr_camera_config,my_config.color_camera_config, my_config.light_config);
    let gpio_config_main = gpio_config.clone();
    

    // 建立一个消息隧道，独立线程接收消息
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut uart = register_gpio(gpio_config).expect("Failed to initialize UART");
        receive_line_loop(&mut uart, tx).expect("Failed to receive data");
    });

    // 处理消息
    let mut uart = register_gpio(gpio_config_main).expect("Failed to initialize UART");
    let (mut color_pin,mut qr_pin,mut gpio_pin) = register_lights(light_config)?;
    loop {
        match rx.recv() {
            Ok(received_data) => {
                gpio_pin.set_low();
                println!("Main thread received: {}", received_data);
                // 根据接收到的数据执行相应的操作
                if received_data == "a1" {
                    color_pin.set_low();
                    println!("Performing operation for a1");
                    let color_name = color_detect_work::work(color_config.clone())?;
                    send_line(&mut uart, &color_name)?;
                    println!("a1 Worked Well: {}",color_name);
                    color_pin.set_high();
                } else if received_data == "b2" {
                    qr_pin.set_low();
                    println!("Performing operation for b2");
                    let task_num = qr_detect_work::work(qr_config.clone())?;
                    println!("b2 Worked Well: {}",task_num);
                    send_line(&mut uart, &task_num.to_string())?;
                    qr_pin.set_high();
                }
                gpio_pin.set_high();
            }
            Err(e) => {
                eprintln!("Failed to receive data: {}", e);
                break;
            }
        }
    }

    // 等待子线程完成
    handle.join().expect("Thread panicked");

    Ok(())
}




#[test]
fn test_color_detect() -> Result<()> {
    let my_config = get_config()?; 
    let color_name = color_detect_work::work(my_config.color_camera_config)?;
    println!("找到最佳颜色 {}",color_name );
   Ok(())
}

#[test]
fn test_qr_detect() -> Result<()>{
    let my_config = get_config()?; 
    let task_num = qr_detect_work::work(my_config.qr_camera_config)?;
    println!("已经识别二维码 {}",task_num);
   Ok(()) 
}
