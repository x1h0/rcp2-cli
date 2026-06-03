use log::warn;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::Value;

/// Sets a boolean property on the `NETWORK` node and updates local state.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn set_bool(conn: &DeviceConnection, name: &str, value: bool) -> rcp2_protocol::Result<()> {
    let network_idx = conn.state().root_child_index("NETWORK")?;
    conn.send_property_update(vec![network_idx], name.into(), Value::Bool(value))?;
    if let Err(e) = conn
        .state()
        .set_property(&[network_idx], name, Value::Bool(value))
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
    let network_idx = conn.state().root_child_index("NETWORK")?;
    conn.send_property_update(
        vec![network_idx],
        "btDoDisconnect".into(),
        Value::String(address.to_string()),
    )
}
