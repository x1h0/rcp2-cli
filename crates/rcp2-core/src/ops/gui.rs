use super::GUI_IDX;
use log::warn;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::Value;

/// Sets a string property on the `GUI` node and updates local state.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn set_string(conn: &DeviceConnection, name: &str, value: &str) -> rcp2_protocol::Result<()> {
    let value = Value::String(value.to_string());
    conn.send_property_update(vec![GUI_IDX], name.into(), value.clone())?;
    if let Err(e) = conn.state().set_property(&[GUI_IDX], name, value) {
        warn!("failed to update local state: {e}");
    }
    Ok(())
}
