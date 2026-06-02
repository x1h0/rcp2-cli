use super::Context;
use rcp2_protocol::device::DeviceEvent;
use std::time::Instant;

pub fn monitor(ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    let conn = super::open_connection(ctx)?;
    println!("monitoring device events (Ctrl+C to stop)...");
    let start = Instant::now();

    for event in conn.events() {
        let t = start.elapsed().as_secs_f64();
        match event {
            DeviceEvent::StateInitialized => {
                println!("[{t:>8.3}] [state] full state received");
            }
            DeviceEvent::PropertyUpdated {
                indices,
                name,
                value,
            } => {
                println!("[{t:>8.3}] [update] {indices:?} {name} = {value:?}");
            }
            DeviceEvent::UnknownPacket(data) => {
                println!(
                    "[{t:>8.3}] [unknown] {} bytes: {:02x?}",
                    data.len(),
                    &data[..std::cmp::min(32, data.len())]
                );
            }
            DeviceEvent::Error(e) => {
                eprintln!("[{t:>8.3}] [error] {e}");
            }
            DeviceEvent::Disconnected => {
                println!("[{t:>8.3}] [disconnected]");
                break;
            }
        }
    }

    Ok(())
}
