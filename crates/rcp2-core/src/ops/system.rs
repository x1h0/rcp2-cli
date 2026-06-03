use log::warn;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::Value;

/// Sets a boolean property on the `SYSTEM` node and updates local state.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn set_bool(conn: &DeviceConnection, name: &str, value: bool) -> rcp2_protocol::Result<()> {
    set_value(conn, name, Value::Bool(value))
}

/// Sets an unsigned-integer property on the `SYSTEM` node and updates local state.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn set_u32(conn: &DeviceConnection, name: &str, value: u32) -> rcp2_protocol::Result<()> {
    set_value(conn, name, Value::U32(value))
}

fn set_value(conn: &DeviceConnection, name: &str, value: Value) -> rcp2_protocol::Result<()> {
    let system_idx = conn.state().root_child_index("SYSTEM")?;
    conn.send_property_update(vec![system_idx], name.into(), value.clone())?;
    if let Err(e) = conn.state().set_property(&[system_idx], name, value) {
        warn!("failed to update local state: {e}");
    }
    Ok(())
}
