use super::Context;
use log::info;
use rcp2_core::DeviceViewModel;
use rcp2_protocol::transport::hid::{HidTransport, PortType};

pub fn connect(ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    let hid_api = hidapi::HidApi::new()?;
    let devices = HidTransport::enumerate(&hid_api);

    if devices.is_empty() {
        println!("no supported R\u{00D8}DECaster devices found");
        return Ok(());
    }

    for device in &devices {
        println!("found: {device}");
    }

    let has_main = devices.iter().any(|d| d.port == PortType::Main);
    let has_secondary = devices.iter().any(|d| d.port == PortType::Secondary);

    if !has_main {
        println!();
        if has_secondary {
            println!("device is connected via the secondary USB-C port.");
            println!("for configuration access, please connect via the main USB-C port.");
        }
        return Ok(());
    }

    info!("opening connection...");
    let conn = super::open_connection(ctx)?;
    conn.wait_for_state()?;

    let state = conn.state().snapshot()?;
    let vm = DeviceViewModel::from_state(&state, conn.model().profile());
    println!("connected: root node = '{}'", state.name);
    if !vm.system.firmware.is_empty() {
        println!("  firmware: {}", vm.system.firmware);
    }
    println!("  properties: {}", state.properties.len());
    println!("  children: {}", state.children.len());

    Ok(())
}
