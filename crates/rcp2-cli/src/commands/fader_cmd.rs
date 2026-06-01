use super::Context;
use rcp2_core::ops::fader;
use rcp2_core::{DeviceViewModel, FaderInfo};

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum OnOffToggle {
    On,
    Off,
    Toggle,
}

impl OnOffToggle {
    fn resolve(self, current: bool) -> bool {
        match self {
            OnOffToggle::On => true,
            OnOffToggle::Off => false,
            OnOffToggle::Toggle => !current,
        }
    }
}

#[derive(clap::Subcommand)]
pub enum FaderAction {
    /// List all faders with mute/listen state and level
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set or toggle mute for fader N (0-based)
    Mute {
        index: usize,
        #[arg(value_enum, default_value = "toggle")]
        state: OnOffToggle,
    },
    /// Set or toggle the Listen (headphone) button for fader N (0-based)
    Listen {
        index: usize,
        #[arg(value_enum, default_value = "toggle")]
        state: OnOffToggle,
    },
}

/// # Errors
/// Returns an error if the connection fails, the fader index is out of range,
/// or a property update fails.
pub fn fader(ctx: &Context, action: &FaderAction) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, state, vm) = super::connect_and_snapshot(ctx)?;
    let channels = fader::channel_indices(&state);

    match action {
        FaderAction::List { json } => {
            if *json {
                let faders: Vec<_> = vm
                    .faders
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        serde_json::json!({
                            "index": i,
                            "configured": f.configured,
                            "mute": f.mute,
                            "listen": f.cue,
                            "level": f.percent(),
                        })
                    })
                    .collect();
                println!("{}", serde_json::Value::Array(faders));
            } else {
                for (i, f) in vm.faders.iter().enumerate() {
                    println!(
                        "{i}: {:<10} mute={} listen={} level={:.0}%",
                        if f.configured { "configured" } else { "unused" },
                        f.mute,
                        f.cue,
                        f.percent() * 100.0,
                    );
                }
            }
        }
        FaderAction::Mute { index, state } => {
            let (channel_index, current) = resolve_fader(&vm, &channels, *index, |f| f.mute)?;
            let target = state.resolve(current);
            fader::set_mute(&conn, channel_index, target)?;
            conn.flush()?;
            println!("fader {index} mute = {target}");
        }
        FaderAction::Listen { index, state } => {
            let (channel_index, current) = resolve_fader(&vm, &channels, *index, |f| f.cue)?;
            let target = state.resolve(current);
            fader::set_listen(&conn, channel_index, target)?;
            conn.flush()?;
            println!("fader {index} listen = {target}");
        }
    }

    Ok(())
}

fn resolve_fader(
    vm: &DeviceViewModel,
    channels: &[usize],
    index: usize,
    field: impl Fn(&FaderInfo) -> bool,
) -> Result<(usize, bool), Box<dyn std::error::Error>> {
    let info = vm
        .faders
        .get(index)
        .ok_or_else(|| format!("fader {index} out of range (0..{})", vm.faders.len()))?;
    let channel_index = *channels
        .get(index)
        .ok_or_else(|| format!("no channel node for fader {index}"))?;
    Ok((channel_index, field(info)))
}
