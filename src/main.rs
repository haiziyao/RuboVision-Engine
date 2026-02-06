use anyhow::{Ok, Result};

mod device;



fn main() -> Result<()> {
    device::camera::main()?;
   Ok(())
}
