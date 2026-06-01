use super::Context;
use super::interactive::{RawModeGuard, poll_key};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use rcp2_core::ops::transfer::{self, TransferState};
use rcp2_core::ops::{TRANSFER_MODE_EMMC, TRANSFER_MODE_SD, pad};
use rcp2_protocol::device::{DeviceConnection, DeviceEvent};
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum StorageArg {
    Emmc,
    Sd,
}

#[derive(clap::Subcommand)]
pub enum TransferAction {
    Interactive {
        /// Storage to mount; prompts interactively if omitted
        #[arg(long, value_enum)]
        storage: Option<StorageArg>,
    },
}

/// # Errors
/// Returns an error if the tools are missing, the connection fails, or
/// activation fails.
pub fn transfer(ctx: &Context, action: &TransferAction) -> Result<(), Box<dyn std::error::Error>> {
    if ctx.dry_run {
        eprintln!("transfer is disabled in dry-run mode");
        return Ok(());
    }

    let TransferAction::Interactive { storage } = action;

    if !transfer::tools_available() {
        return Err("transfer requires lsblk and udisksctl".into());
    }

    let conn = super::open_connection(ctx)?;
    conn.wait_for_state()?;
    let state = conn.state().snapshot()?;
    let vm = rcp2_core::DeviceViewModel::from_state(&state, conn.model().profile());

    let mode = match storage {
        Some(StorageArg::Sd) => TRANSFER_MODE_SD,
        Some(StorageArg::Emmc) => TRANSFER_MODE_EMMC,
        None => prompt_storage(&vm)?,
    };

    if mode == TRANSFER_MODE_SD && !vm.has_storage() {
        return Err("no SD card detected, insert a card and try again".into());
    }

    let label = if mode == TRANSFER_MODE_SD { "sd" } else { "emmc" };
    pad::activate_transfer_mode(&conn, mode)?;
    println!("activating transfer mode ({label})...");

    let mut guard = TransferGuard {
        conn: &conn,
        ts: TransferState::new(),
    };
    transfer_session(&conn, &mut guard.ts)
}

fn prompt_storage(
    vm: &rcp2_core::DeviceViewModel,
) -> Result<u32, Box<dyn std::error::Error>> {
    let sd = if vm.has_storage() {
        "SD Card (recordings, scene exports)"
    } else {
        "SD Card (no card inserted)"
    };
    println!("Select storage:");
    println!("  1  Internal eMMC (pads, system data)");
    println!("  2  {sd}");
    print!("> ");
    std::io::stdout().flush()?;

    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    match line.trim() {
        "1" | "emmc" => Ok(TRANSFER_MODE_EMMC),
        "2" | "sd" => Ok(TRANSFER_MODE_SD),
        other => Err(format!("invalid choice '{other}' (enter 1 or 2)").into()),
    }
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
