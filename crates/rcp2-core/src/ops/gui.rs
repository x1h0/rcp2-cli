use log::warn;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::Value;

/// Sets a string property on the `GUI` node and updates local state.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn set_string(conn: &DeviceConnection, name: &str, value: &str) -> rcp2_protocol::Result<()> {
    let gui_idx = conn.state().root_child_index("GUI")?;
    let value = Value::String(value.to_string());
    conn.send_property_update(vec![gui_idx], name.into(), value.clone())?;
    if let Err(e) = conn.state().set_property(&[gui_idx], name, value) {
        warn!("failed to update local state: {e}");
    }
    Ok(())
}
