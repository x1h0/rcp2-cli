use super::Context;
use super::interactive::{RawModeGuard, poll_key};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use rcp2_core::RecordingStatus;
use rcp2_core::ops::recorder;
use rcp2_protocol::device::{DeviceConnection, DeviceEvent};
use rcp2_protocol::types::Value;
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(clap::Subcommand)]
pub enum RecordAction {
    /// Show current recording status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Interactive control with live timer (keys: space/r start/resume, p pause, s stop, q quit)
    Interactive,
}

/// # Errors
/// Returns an error if the device connection or a property update fails.
pub fn record(ctx: &Context, action: &RecordAction) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, _state, vm) = super::connect_and_snapshot(ctx)?;

    match action {
        RecordAction::Status { json } => {
            let state = status_key(vm.recorder.state);
            if *json {
                println!("{}", serde_json::json!({ "state": state }));
            } else {
                println!("{}", vm.recorder.state.label());
            }
        }
        RecordAction::Interactive => {
            if vm.recorder.state != RecordingStatus::Stopped {
                println!(
                    "recording already in progress ({}); stop it on the device first \
                     (the device does not report elapsed time)",
                    vm.recorder.state.label(),
                );
                return Ok(());
            }
            record_interactive(&conn)?;
        }
    }

    Ok(())
}

fn status_key(status: RecordingStatus) -> &'static str {
    match status {
        RecordingStatus::Stopped => "stopped",
        RecordingStatus::Recording => "recording",
        RecordingStatus::Paused => "paused",
    }
}

fn fmt_hms(total: u64) -> String {
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

fn key_hint(status: RecordingStatus) -> &'static str {
    match status {
        RecordingStatus::Stopped => "[space/r] start  [q] quit",
        RecordingStatus::Recording => "[p] pause  [s] stop  [q] quit",
        RecordingStatus::Paused => "[space/r] resume  [s] stop  [q] quit",
    }
}

fn record_interactive(conn: &DeviceConnection) -> Result<(), Box<dyn std::error::Error>> {
    let _guard = RawModeGuard::enter()?;
    let mut status = RecordingStatus::Stopped;
    let mut started: Option<Instant> = None;
    let mut base_secs = 0;
    let mut stop_requested = false;

    loop {
        let prev = status;
        for _ in 0..256 {
            let Ok(event) = conn.events().try_recv() else {
                break;
            };
            match event {
                DeviceEvent::PropertyUpdated { name, value, .. } => {
                    if name == "recordState"
                        && let Value::U32(v) = value
                    {
                        status = RecordingStatus::from_u32(v);
                    }
                }
                DeviceEvent::Disconnected => {
                    print!("\r\ndevice disconnected\r\n");
                    std::io::stdout().flush()?;
                    return Ok(());
                }
                _ => {}
            }
        }

        if prev != status {
            match status {
                RecordingStatus::Recording => started = Some(Instant::now()),
                RecordingStatus::Paused => {
                    if let Some(s) = started {
                        base_secs += s.elapsed().as_secs();
                    }
                    started = None;
                }
                RecordingStatus::Stopped => {
                    started = None;
                    base_secs = 0;
                }
            }
        }

        if stop_requested && status == RecordingStatus::Stopped {
            break;
        }

        let secs = base_secs + started.map_or(0, |s| s.elapsed().as_secs());
        render(status, secs, conn.is_dry_run());

        let Some(key) = poll_key(Duration::from_millis(100))? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if matches!(status, RecordingStatus::Recording | RecordingStatus::Paused) {
                    let _ = recorder::stop_recording(conn);
                }
                break;
            }
            KeyCode::Char(' ' | 'r') => {
                let _ = recorder::start_recording(conn);
            }
            KeyCode::Char('p') => {
                if status == RecordingStatus::Recording {
                    let _ = recorder::pause_recording(conn);
                }
            }
            KeyCode::Char('s') => {
                if matches!(status, RecordingStatus::Recording | RecordingStatus::Paused) {
                    let _ = recorder::stop_recording(conn);
                    stop_requested = true;
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                if matches!(status, RecordingStatus::Recording | RecordingStatus::Paused) {
                    let _ = recorder::stop_recording(conn);
                }
                break;
            }
            _ => {}
        }
    }

    let _ = conn.flush();
    print!("\r\n");
    std::io::stdout().flush()?;
    Ok(())
}

fn render(status: RecordingStatus, secs: u64, dry_run: bool) {
    print!(
        "\r{}{:<5} {}   {:<38}",
        if dry_run { "[dry-run] " } else { "" },
        status.label(),
        fmt_hms(secs),
        key_hint(status),
    );
    let _ = std::io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_hms_minutes() {
        assert_eq!(fmt_hms(0), "00:00");
        assert_eq!(fmt_hms(12), "00:12");
        assert_eq!(fmt_hms(75), "01:15");
    }

    #[test]
    fn fmt_hms_hours() {
        assert_eq!(fmt_hms(3661), "01:01:01");
    }

    #[test]
    fn status_key_values() {
        assert_eq!(status_key(RecordingStatus::Stopped), "stopped");
        assert_eq!(status_key(RecordingStatus::Recording), "recording");
        assert_eq!(status_key(RecordingStatus::Paused), "paused");
    }
}
