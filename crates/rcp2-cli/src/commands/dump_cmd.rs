use super::Context;
use rcp2_protocol::types::{Structured, Value};

pub fn dump(ctx: &Context, full: bool) -> Result<(), Box<dyn std::error::Error>> {
    let conn = super::open_connection(ctx)?;
    conn.wait_for_state()?;
    let mut state = conn.state().snapshot()?;

    if !full {
        redact(&mut state);
    }

    let json = serde_json::to_string_pretty(&state)?;
    println!("{json}");

    Ok(())
}

fn redact(node: &mut Structured) {
    for (name, value) in &mut node.properties {
        if is_sensitive(name)
            && let Value::String(s) = value
            && !s.is_empty()
        {
            *s = "REDACTED".into();
        }
    }
    for child in &mut node.children {
        redact(child);
    }
}

fn is_sensitive(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "systemSerialNumber",
        "ipAddress",
        "gateway",
        "sipAccountPassword",
        "sipWebPassword",
        "sipNATTraversalPassword",
        "sipAccountUsername",
        "sipAccountAuthUsername",
        "sipNATTraversalUsername",
        "sipAccountDomain",
        "sipAccountProxyAddress",
        "sipNATTraversalServer",
        "sipUnitName",
        "sipRodeCode",
        "sipSlotCallAddress",
        "sipSlotCallUUID",
        "sipRegistrationDetails",
        "sipIncomingCallDetails",
        "sipOutgoingCallDetails",
    ];
    const PREFIX: &[&str] = &["wifi", "btScan", "btConnected", "btPair"];
    EXACT.contains(&name) || PREFIX.iter().any(|p| name.starts_with(p))
}
