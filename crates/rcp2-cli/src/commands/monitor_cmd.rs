use super::Context;
use rcp2_protocol::device::DeviceEvent;

pub fn monitor(ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    if ctx.offline {
        println!("offline mode: nothing to monitor");
        return Ok(());
    }

    let conn = super::open_connection(ctx)?;
    println!("monitoring device events (Ctrl+C to stop)...");

    for event in conn.events() {
        match event {
            DeviceEvent::StateInitialized => {
                println!("[state] full state received");
            }
            DeviceEvent::PropertyUpdated {
                indices,
                name,
                value,
            } => {
                println!("[update] {indices:?} {name} = {value:?}");
            }
            DeviceEvent::UnknownPacket(data) => {
                println!(
                    "[unknown] {} bytes: {:02x?}",
                    data.len(),
                    &data[..std::cmp::min(32, data.len())]
                );
            }
            DeviceEvent::Error(e) => {
                eprintln!("[error] {e}");
            }
            DeviceEvent::Disconnected => {
                println!("[disconnected]");
                break;
            }
        }
    }

    Ok(())
}
