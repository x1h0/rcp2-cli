use super::Context;
use super::interactive::{RawModeGuard, poll_key};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use rcp2_core::ops::transfer::{self, TransferState};
use rcp2_core::ops::{TRANSFER_MODE_EMMC, TRANSFER_MODE_SD, pad};
use rcp2_protocol::device::{DeviceConnection, DeviceEvent};
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(clap::Subcommand)]
pub enum TransferAction {
    Interactive {
        #[arg(long, default_value = "emmc")]
        storage: String,
    },
}

/// # Errors
/// Returns an error if the tools are missing, the connection fails, or
/// activation fails.
pub fn transfer(ctx: &Context, action: &TransferAction) -> Result<(), Box<dyn std::error::Error>> {
    let TransferAction::Interactive { storage } = action;
    let mode = match storage.as_str() {
        "sd" => TRANSFER_MODE_SD,
        "emmc" => TRANSFER_MODE_EMMC,
        other => return Err(format!("unknown storage '{other}' (use emmc or sd)").into()),
    };

    if !transfer::tools_available() {
        return Err("transfer requires lsblk and udisksctl".into());
    }

    let conn = super::open_connection(ctx)?;
    conn.wait_for_state()?;
    pad::activate_transfer_mode(&conn, mode)?;
    println!("activating transfer mode ({storage})...");

    let mut guard = TransferGuard {
        conn: &conn,
        ts: TransferState::new(),
    };
    transfer_session(&conn, &mut guard.ts)
}

struct TransferGuard<'a> {
    conn: &'a DeviceConnection,
    ts: TransferState,
}

impl Drop for TransferGuard<'_> {
    fn drop(&mut self) {
        println!("deactivating transfer mode...");
        self.ts.unmount();
        if let Err(e) = pad::deactivate_transfer_mode(self.conn) {
            eprintln!("warning: failed to deactivate transfer mode: {e}");
        }
        let _ = self.conn.flush();
    }
}

fn transfer_session(
    conn: &DeviceConnection,
    ts: &mut TransferState,
) -> Result<(), Box<dyn std::error::Error>> {
    const MOUNT_TIMEOUT: Duration = Duration::from_secs(20);
    let started = Instant::now();
    print!("waiting for mount");
    std::io::stdout().flush()?;

    while !ts.find_mount_point() {
        if started.elapsed() > MOUNT_TIMEOUT {
            println!();
            return Err("timed out waiting for device to mount".into());
        }
        print!(".");
        std::io::stdout().flush()?;
        std::thread::sleep(Duration::from_millis(500));
    }

    let mount = ts.mount_point.clone().unwrap_or_default();
    println!("\ntransfer mode active, mounted at: {mount}");

    keep_open(conn)
}

fn keep_open(conn: &DeviceConnection) -> Result<(), Box<dyn std::error::Error>> {
    let _guard = RawModeGuard::enter()?;
    print!("\rtransfer mode active  [q] quit    ");
    std::io::stdout().flush()?;

    loop {
        while let Ok(event) = conn.events().try_recv() {
            if matches!(event, DeviceEvent::Disconnected) {
                print!("\r\ndevice disconnected\r\n");
                std::io::stdout().flush()?;
                return Ok(());
            }
        }

        let Some(key) = poll_key(Duration::from_millis(200))? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
            KeyCode::Char('q') | KeyCode::Esc => break,
            _ => {}
        }
    }

    print!("\r\n");
    std::io::stdout().flush()?;
    Ok(())
}
