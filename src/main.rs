//! SigmaOS component binary: live-media

use sigmaos_live-media::Component;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Component::new().operate()?;
    println!("sigmaos-live-media: ok");
    Ok(())
}
