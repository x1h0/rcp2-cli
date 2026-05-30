use super::RECORDER_IDX;
use log::info;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::Value;

const REQUEST_RECORD: u32 = 2;
const REQUEST_PAUSE: u32 = 1;
const REQUEST_STOP: u32 = 0;

/// Starts or resumes recording on the device.
///
/// # Errors
/// Returns an error if the property update cannot be sent.
pub fn start_recording(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("requesting record start");
    conn.send_property_update(
        vec![RECORDER_IDX],
        "requestRecordState".into(),
        Value::U32(REQUEST_RECORD),
    )
}

/// Pauses a running recording.
///
/// # Errors
/// Returns an error if the property update cannot be sent.
pub fn pause_recording(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("requesting record pause");
    conn.send_property_update(
        vec![RECORDER_IDX],
        "requestRecordState".into(),
        Value::U32(REQUEST_PAUSE),
    )
}

/// Stops recording.
///
/// # Errors
/// Returns an error if the property update cannot be sent.
pub fn stop_recording(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("requesting record stop");
    conn.send_property_update(
        vec![RECORDER_IDX],
        "requestRecordState".into(),
        Value::U32(REQUEST_STOP),
    )
}
