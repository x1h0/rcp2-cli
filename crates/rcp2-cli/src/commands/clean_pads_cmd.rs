use super::Context;
use rcp2_protocol::packet::child_removed::ChildRemovedPacket;

pub fn clean_pads(ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    if ctx.offline || ctx.dry_run {
        println!("clean-pads requires a connected device");
        return Ok(());
    }

    let conn = super::open_connection(ctx)?;
    conn.wait_for_state()?;

    let state = conn.state().snapshot()?;

    let sp_idx = state
        .children
        .iter()
        .position(|c| c.name == "SOUNDPADS")
        .ok_or("SOUNDPADS not found")?;

    let soundpads = &state.children[sp_idx];
    let mut broken: Vec<usize> = Vec::new();

    for (i, child) in soundpads.children.iter().enumerate() {
        if !child.properties.contains_key("padIdx") {
            println!("broken pad at child index {i} (no padIdx)");
            broken.push(i);
        }
    }

    if broken.is_empty() {
        println!("no broken pads found");
        return Ok(());
    }

    println!("found {} broken pad(s), removing...", broken.len());

    for &child_idx in broken.iter().rev() {
        println!("  removing child {child_idx}...");
        let packet = ChildRemovedPacket {
            path: vec![sp_idx],
            child_index: child_idx,
        };
        conn.send_packet(Box::new(packet))?;
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    conn.flush()?;
    println!("done. {} broken pad(s) removed.", broken.len());
    Ok(())
}
