#[cfg(debug_assertions)]
mod clean_pads_cmd;
mod connect_cmd;
mod dump_cmd;
mod fader_cmd;
mod interactive;
mod monitor_cmd;
mod record_cmd;
#[cfg(debug_assertions)]
mod send_cmd;
#[cfg(debug_assertions)]
mod set_property_cmd;
mod transfer_cmd;

use rcp2_core::DeviceViewModel;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::transport::hid::HidTransport;
use rcp2_protocol::types::Structured;
use std::io::Write;

pub struct Context {
    pub dry_run: bool,
    pub accepted: bool,
}

pub(crate) fn dry_run_suffix(ctx: &Context) -> &'static str {
    if ctx.dry_run { " (dry-run)" } else { "" }
}

#[cfg(debug_assertions)]
pub use clean_pads_cmd::clean_pads;
pub use connect_cmd::connect;
pub use dump_cmd::dump;
pub use fader_cmd::{FaderAction, fader};
pub use monitor_cmd::monitor;
pub use record_cmd::{RecordAction, record};
#[cfg(debug_assertions)]
pub use send_cmd::send;
#[cfg(debug_assertions)]
pub use set_property_cmd::set_property;
pub use transfer_cmd::{TransferAction, transfer};

pub fn run_with_disclaimer(
    ctx: &Context,
    f: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !ctx.accepted {
        cli_disclaimer()?;
    }
    f()
}

fn cli_disclaimer() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("WARNING: This tool communicates with your R\u{00D8}DECaster device");
    eprintln!("via USB HID using a reverse-engineered protocol.");
    eprintln!();
    eprintln!("Known issue: after closing, device buttons may freeze.");
    eprintln!("Replug the USB cable to recover.");
    eprintln!();
    eprintln!("No warranty. Use at your own risk.");
    eprintln!();
    eprint!("Continue? [y/N] ");
    std::io::stderr().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        Ok(())
    } else {
        Err("aborted".into())
    }
}

fn open_connection(ctx: &Context) -> Result<DeviceConnection, Box<dyn std::error::Error>> {
    let hid_api = hidapi::HidApi::new()?;
    let ((rx, tx), model) = HidTransport::open_pair(&hid_api)?;
    let conn = DeviceConnection::open(Box::new(rx), Box::new(tx), model, ctx.dry_run)?;
    Ok(conn)
}

fn connect_and_snapshot(
    ctx: &Context,
) -> Result<(DeviceConnection, Structured, DeviceViewModel), Box<dyn std::error::Error>> {
    let conn = open_connection(ctx)?;
    conn.wait_for_state()?;
    let state = conn.state().snapshot()?;
    let vm = DeviceViewModel::from_state(&state, conn.model().profile());
    Ok((conn, state, vm))
}
