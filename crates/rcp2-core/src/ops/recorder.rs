use log::info;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::Value;

const REQUEST_RECORD: u32 = 2;
const REQUEST_PAUSE: u32 = 1;
const REQUEST_STOP: u32 = 0;

fn request_record_state(conn: &DeviceConnection, state: u32) -> rcp2_protocol::Result<()> {
    let recorder_idx = conn.state().root_child_index("RECORDER")?;
    conn.send_property_update(
        vec![recorder_idx],
        "requestRecordState".into(),
        Value::U32(state),
    )
}

/// Starts or resumes recording on the device.
///
/// # Errors
/// Returns an error if the property update cannot be sent.
pub fn start_recording(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("requesting record start");
    request_record_state(conn, REQUEST_RECORD)
}

/// Pauses a running recording.
///
/// # Errors
/// Returns an error if the property update cannot be sent.
pub fn pause_recording(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("requesting record pause");
    request_record_state(conn, REQUEST_PAUSE)
}

/// Stops recording.
///
/// # Errors
/// Returns an error if the property update cannot be sent.
pub fn stop_recording(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("requesting record stop");
    request_record_state(conn, REQUEST_STOP)
}
