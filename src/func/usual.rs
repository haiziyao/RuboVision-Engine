use std::thread::sleep;
use std::time::Duration;
use tracing::field::debug;
use crate::device::Device;
use crate::web::{WebMessage};

pub fn example_fn( ) -> Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send> {
    Box::new( |args,device| {
        WebMessage::ok("hello from FnOnce")
    })
}


pub fn fn_debug() -> Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send> {
    debug("debug Function executing");
    Box::new( move |args,device| {
        sleep(Duration::from_secs(5));
        let args = args.join(",");
        WebMessage::ok(format!("this is the debug function {args}"))
    })
}
 