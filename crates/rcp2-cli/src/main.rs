mod commands;

use clap::Parser;
use log::LevelFilter;
use std::io::{IsTerminal, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for LogBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut inner) = self.0.lock() {
            inner.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct FlushOnDrop(Option<Arc<Mutex<Vec<u8>>>>);

impl Drop for FlushOnDrop {
    fn drop(&mut self) {
        if let Some(buffer) = &self.0
            && let Ok(logs) = buffer.lock()
            && !logs.is_empty()
        {
            let _ = std::io::stderr().write_all(&logs);
        }
    }
}

#[derive(Parser)]
#[command(name = "rcp2-cli", about = "R\u{00D8}DECaster management tool")]
struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Dry-run mode: connect and read normally, but log writes instead of sending them
    #[arg(long, global = true)]
    dry_run: bool,

    /// Skip the disclaimer prompt (implies you accept all risks)
    #[arg(long, global = true)]
    i_know_what_i_do: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Connect to device and show info
    Connect,
    /// Dump full device state tree as JSON
    Dump,
    /// Monitor property updates in real-time
    Monitor,
    /// Control recording (status/interactive)
    Record {
        #[command(subcommand)]
        action: commands::RecordAction,
    },
    /// File transfer mode (emmc/sd)
    Transfer {
        #[command(subcommand)]
        action: commands::TransferAction,
    },
    /// Fader mute / listen control
    Fader {
        #[command(subcommand)]
        action: commands::FaderAction,
    },
    /// Soundpad control (trigger)
    Pad {
        #[command(subcommand)]
        action: commands::PadAction,
    },
    /// Send raw hex bytes to device (dev builds only)
    #[cfg(debug_assertions)]
    Send {
        /// Hex string (e.g. "AD10A7B0")
        hex: String,
    },
    /// Set a property on the device (dev builds only)
    #[cfg(debug_assertions)]
    SetProperty {
        /// Comma-separated indices (e.g. "13" or "13,2")
        indices: String,
        /// Property name
        name: String,
        /// Value (u32:N, bool:true/false, f64:N, str:text)
        value: String,
    },
    /// Remove broken pad nodes (no padIdx property) from SOUNDPADS (dev builds only)
    #[cfg(debug_assertions)]
    CleanPads,
    /// Launch interactive TUI
    Tui,
}

fn main() {
    let cli = Cli::parse();

    let mut log_level = match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    if cli.dry_run {
        log_level = log_level.max(LevelFilter::Info);
    }

    let is_tui = matches!(cli.command, Commands::Tui);
    let mut builder = env_logger::Builder::new();
    builder.filter_level(log_level).format_timestamp_millis();

    let buffered_logs = if is_tui && std::io::stderr().is_terminal() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        builder.target(env_logger::Target::Pipe(Box::new(LogBuffer(buffer.clone()))));
        Some(buffer)
    } else {
        None
    };
    builder.init();

    let accepted = cli.i_know_what_i_do || std::env::var("RCP2_ACCEPT_RISK").as_deref() == Ok("1");

    let ctx = commands::Context {
        dry_run: cli.dry_run,
        accepted,
    };

    let flush_guard = FlushOnDrop(buffered_logs);

    let result = match cli.command {
        Commands::Connect => commands::run_with_disclaimer(&ctx, || commands::connect(&ctx)),
        Commands::Dump => commands::run_with_disclaimer(&ctx, || commands::dump(&ctx)),
        Commands::Monitor => commands::run_with_disclaimer(&ctx, || commands::monitor(&ctx)),
        Commands::Record { ref action } => {
            commands::run_with_disclaimer(&ctx, || commands::record(&ctx, action))
        }
        Commands::Transfer { ref action } => {
            commands::run_with_disclaimer(&ctx, || commands::transfer(&ctx, action))
        }
        Commands::Fader { ref action } => {
            commands::run_with_disclaimer(&ctx, || commands::fader(&ctx, action))
        }
        Commands::Pad { ref action } => {
            commands::run_with_disclaimer(&ctx, || commands::pad(&ctx, action))
        }
        #[cfg(debug_assertions)]
        Commands::Send { ref hex } => {
            commands::run_with_disclaimer(&ctx, || commands::send(&ctx, hex))
        }
        #[cfg(debug_assertions)]
        Commands::SetProperty {
            ref indices,
            ref name,
            ref value,
        } => commands::run_with_disclaimer(&ctx, || {
            commands::set_property(&ctx, indices, name, value)
        }),
        #[cfg(debug_assertions)]
        Commands::CleanPads => commands::run_with_disclaimer(&ctx, || commands::clean_pads(&ctx)),
        Commands::Tui => rcp2_tui::run(cli.dry_run, accepted),
    };

    if let Err(e) = result {
        drop(flush_guard);
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
