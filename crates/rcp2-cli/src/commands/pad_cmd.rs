use super::{Context, dry_run_suffix};
use rcp2_core::BankView;
use rcp2_core::ops::pad as pad_ops;
use std::time::Duration;

const BANK_SETTLE_DELAY: Duration = Duration::from_millis(200);

#[derive(clap::Subcommand)]
pub enum PadAction {
    /// Trigger (tap) the pad at BANK and PAD (both 0-based, PAD in grid order)
    Trigger {
        bank: usize,
        pad: usize,
        /// How long to hold the pad, in milliseconds (default 50)
        #[arg(long)]
        hold: Option<u64>,
        /// Keep the triggered bank selected instead of restoring the previous one
        #[arg(long)]
        no_restore: bool,
    },
}

/// # Errors
/// Returns an error if the connection fails, the bank or pad is out of range,
/// or a property update fails.
pub fn pad(ctx: &Context, action: &PadAction) -> Result<(), Box<dyn std::error::Error>> {
    let PadAction::Trigger {
        bank,
        pad,
        hold,
        no_restore,
    } = action;

    let (conn, _, vm) = super::connect_and_snapshot(ctx)?;
    let profile = vm.profile;

    if *bank >= profile.max_banks {
        return Err(format!("bank {bank} out of range (0..{})", profile.max_banks).into());
    }
    if *pad >= profile.pads_per_bank {
        return Err(format!("pad {pad} out of range (0..{})", profile.pads_per_bank).into());
    }

    let previous_bank = vm.selected_bank;
    let position = BankView::logical_index(*pad, profile);
    let switch = *bank != previous_bank;

    if switch {
        pad_ops::sync_bank(&conn, *bank)?;
        conn.flush()?;
        std::thread::sleep(BANK_SETTLE_DELAY);
    }

    match hold {
        Some(ms) => pad_ops::tap_pad_for(&conn, position, profile, Duration::from_millis(*ms))?,
        None => pad_ops::tap_pad(&conn, position, profile)?,
    }

    if switch && !no_restore {
        std::thread::sleep(BANK_SETTLE_DELAY);
        pad_ops::sync_bank(&conn, previous_bank)?;
    }

    conn.flush()?;
    println!("triggered bank {bank} pad {pad}{}", dry_run_suffix(ctx));
    Ok(())
}
