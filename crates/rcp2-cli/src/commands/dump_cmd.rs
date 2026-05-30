use super::Context;
use rcp2_protocol::types::Structured;
use std::path::Path;

pub fn dump(ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    let state = if ctx.offline {
        load_offline_state(ctx)?
    } else {
        let conn = super::open_connection(ctx)?;
        conn.wait_for_state()?;
        conn.state().snapshot()?
    };

    let json = serde_json::to_string_pretty(&state)?;
    println!("{json}");

    Ok(())
}

fn load_offline_state(ctx: &Context) -> Result<Structured, Box<dyn std::error::Error>> {
    let path = ctx
        .state_file
        .as_deref()
        .ok_or("--offline requires --state-file <path>")?;

    if !Path::new(path).exists() {
        return Err(format!("state file not found: {path}").into());
    }

    let data = std::fs::read_to_string(path)?;
    let state: Structured = serde_json::from_str(&data)?;
    Ok(state)
}
