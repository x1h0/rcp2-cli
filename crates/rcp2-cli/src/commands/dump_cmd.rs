use super::Context;

pub fn dump(ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    let conn = super::open_connection(ctx)?;
    conn.wait_for_state()?;
    let state = conn.state().snapshot()?;

    let json = serde_json::to_string_pretty(&state)?;
    println!("{json}");

    Ok(())
}
