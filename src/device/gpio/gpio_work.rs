#![allow(dead_code)]
use anyhow::{ Result};
use crate::config::device_config::{GpioConfig, LightConfig};
use rppal::uart::{Uart,Parity,Queue};
use std::time::Duration;
use std::sync::mpsc;
use rppal::gpio::{Gpio,OutputPin};
pub fn register_gpio(config:GpioConfig) ->Result<Uart> {
    let mut uart = Uart::with_path(config.clone().serial, config.baud, Parity::None, config.data_bit, config.stop_bit)?;
    uart.set_read_mode(0, Duration::from_millis(100))?;
    Ok(uart)
}

pub fn send_line(uart:&mut Uart, s: &str) ->Result<()> {
    uart.write(s.as_bytes())?;
    uart.write(b"\n")?;
    uart.flush(Queue::Output)?;
    Ok(())
}

 
pub fn receive_line(uart:&mut Uart) ->Result<()>{
    let mut buffer = [0u8;4];
    uart.read(&mut buffer)?;
    let _received = String::from_utf8_lossy(&buffer).to_string();
    uart.flush(Queue::Input)?;
    Ok(())
}

pub fn receive_line_loop(uart: &mut Uart, tx: mpsc::Sender<String>) -> Result<()> {
    let mut buffer = [0u8; 2];  
    loop {
        match uart.read(&mut buffer) {
            Ok(2) => {  
                let received = String::from_utf8_lossy(&buffer).to_string();
                tx.send(received)?;  
            }
            Ok(_) => continue,  
            Err(e) => {
                eprintln!("Failed to read data: {}", e);
                break;
            }
        }
       
    }
    Ok(())
}

pub fn register_lights(config:LightConfig) -> Result<(OutputPin,OutputPin,OutputPin)>{
    let gpio = Gpio::new().expect("Failed to access GPIO");
    let mut color_pin: OutputPin = gpio.get(config.color_light_pin).expect("Failed to access pin 17").into_output();
    let mut qr_pin: OutputPin = gpio.get(config.qr_light_pin).expect("Failed to access pin 27").into_output();
    let mut gpio_pin: OutputPin = gpio.get(config.gpio_light_pin).expect("Failed to access pin 22").into_output();
    color_pin.set_high();
    qr_pin.set_high();
    gpio_pin.set_high();
    
    Ok((color_pin,qr_pin,gpio_pin))
}