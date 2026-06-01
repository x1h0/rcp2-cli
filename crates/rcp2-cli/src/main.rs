mod commands;

use clap::Parser;
use log::LevelFilter;

#[derive(Parser)]
#[command(name = "rcp2-cli", about = "R\u{00D8}DECaster management tool")]
struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Dry-run mode: show what would be sent without writing to device
    #[arg(long, global = true)]
    dry_run: bool,

    /// Offline mode: no USB communication at all
    #[arg(long, global = true)]
    offline: bool,

    /// Path to a JSON state file (for --offline mode)
    #[arg(long, global = true)]
    state_file: Option<String>,

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
    Tui {
        /// Enable send commands (pad trigger, edit pads, transfer)
        #[arg(long)]
        allow_send: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let log_level = match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let is_tui = matches!(cli.command, Commands::Tui { .. });

    if !is_tui {
        env_logger::Builder::new()
            .filter_level(log_level)
            .format_timestamp_millis()
            .init();
    }

    let accepted = cli.i_know_what_i_do || std::env::var("RCP2_ACCEPT_RISK").as_deref() == Ok("1");

    let ctx = commands::Context {
        dry_run: cli.dry_run,
        offline: cli.offline,
        state_file: cli.state_file,
        accepted,
    };

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
        Commands::Tui { allow_send } => {
            let send = allow_send || std::env::var("RCP2_ALLOW_SEND").as_deref() == Ok("1");
            rcp2_tui::run(send, accepted)
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
