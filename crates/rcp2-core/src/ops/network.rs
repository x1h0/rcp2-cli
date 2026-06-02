use super::NETWORK_IDX;
use log::warn;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::Value;

/// Sets a boolean property on the `NETWORK` node and updates local state.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn set_bool(conn: &DeviceConnection, name: &str, value: bool) -> rcp2_protocol::Result<()> {
    conn.send_property_update(vec![NETWORK_IDX], name.into(), Value::Bool(value))?;
    if let Err(e) = conn
        .state()
        .set_property(&[NETWORK_IDX], name, Value::Bool(value))
    {
        warn!("failed to update local state: {e}");
    }
    Ok(())
}

/// Requests a Bluetooth disconnect. The device clears the connection in its own
/// state update shortly after, which the caller observes to leave the disconnecting state.
///
/// # Errors
/// Returns an error if sending the disconnect request fails.
pub fn disconnect_bluetooth(conn: &DeviceConnection, address: &str) -> rcp2_protocol::Result<()> {
    conn.send_property_update(
        vec![NETWORK_IDX],
        "btDoDisconnect".into(),
        Value::String(address.to_string()),
    )
}
