use super::{GUI_IDX, SYSTEM_IDX};
use log::{info, warn};
use rcp2_protocol::device::{DeviceConnection, DeviceProfile, PHYSICAL_INTERFACE_IDX};
use rcp2_protocol::packet::child_added::ChildAddedPacket;
use rcp2_protocol::types::Value;
use std::time::Duration;

const BUTTON_PRESS_DELAY: Duration = Duration::from_millis(50);
const NODE_CREATION_DELAY: Duration = Duration::from_millis(300);
const PROPERTY_UPDATE_DELAY: Duration = Duration::from_millis(30);

/// Simulates a tap on a SMART pad button.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn tap_pad(
    conn: &DeviceConnection,
    pad_position: usize,
    profile: &DeviceProfile,
) -> rcp2_protocol::Result<()> {
    tap_pad_for(conn, pad_position, profile, BUTTON_PRESS_DELAY)
}

/// Simulates a tap on a SMART pad button, holding it for the given duration.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn tap_pad_for(
    conn: &DeviceConnection,
    pad_position: usize,
    profile: &DeviceProfile,
    hold: Duration,
) -> rcp2_protocol::Result<()> {
    let button_idx = profile.padbutton_offset + pad_position;
    let indices = vec![PHYSICAL_INTERFACE_IDX, button_idx];
    conn.send_property_update(
        indices.clone(),
        "padButtonPressed".into(),
        Value::Bool(true),
    )?;
    std::thread::sleep(hold);
    conn.send_property_update(indices, "padButtonPressed".into(), Value::Bool(false))?;
    Ok(())
}

/// Syncs the selected bank index to the device.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn sync_bank(conn: &DeviceConnection, bank: usize) -> rcp2_protocol::Result<()> {
    let value = Value::U32(u32::try_from(bank).unwrap_or(0));
    conn.send_property_update(vec![GUI_IDX], "selectedBank".into(), value.clone())?;
    if let Err(e) = conn.state().set_property(&[GUI_IDX], "selectedBank", value) {
        warn!("failed to update local state: {e}");
    }
    Ok(())
}

/// Sends a property update to a pad and updates local state.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn send_property(
    conn: &DeviceConnection,
    soundpads_idx: usize,
    pad_child_index: usize,
    name: &str,
    value: Value,
) -> rcp2_protocol::Result<()> {
    let indices = vec![soundpads_idx, pad_child_index];
    conn.send_property_update(indices.clone(), name.into(), value.clone())?;
    if let Err(e) = conn.state().set_property(&indices, name, value) {
        warn!("failed to update local state: {e}");
    }
    Ok(())
}

/// Sends a property update to a pad without updating local state.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn send_property_to_device(
    conn: &DeviceConnection,
    soundpads_idx: usize,
    pad_child_index: usize,
    name: &str,
    value: Value,
) -> rcp2_protocol::Result<()> {
    let indices = vec![soundpads_idx, pad_child_index];
    conn.send_property_update(indices, name.into(), value)
}

/// Creates a new PAD child node on the device with default properties.
///
/// # Errors
/// Returns an error if sending the child-added packet or property updates fails.
pub fn create_pad_node(
    conn: &DeviceConnection,
    soundpads_idx: usize,
    insert_at: usize,
    pad_idx: usize,
    filename: &str,
    pad_name: &str,
) -> rcp2_protocol::Result<()> {
    info!(
        "creating PAD node at SOUNDPADS[{soundpads_idx}] position {insert_at} for padIdx {pad_idx}"
    );

    let packet = ChildAddedPacket {
        path: vec![soundpads_idx],
        insert_index: insert_at,
        node_name: "PAD".into(),
    };
    conn.send_packet(Box::new(packet))?;

    std::thread::sleep(NODE_CREATION_DELAY);

    let device_path = format!("/Application/emmc-data/pads/{}/{}", pad_idx + 1, filename);

    let indices = vec![soundpads_idx, insert_at];
    let props: Vec<(&str, Value)> = vec![
        ("padIdx", Value::U32(u32::try_from(pad_idx).unwrap_or(0))),
        ("padType", Value::U32(1)),
        ("padName", Value::String(pad_name.into())),
        ("padFilePath", Value::String(device_path)),
        (
            "padColourIndex",
            Value::U32(u32::try_from(pad_idx % 12).unwrap_or(0)),
        ),
        ("padGain", Value::F64(-12.0)),
        ("padLoop", Value::Bool(false)),
        ("padReplay", Value::Bool(true)),
        ("padActive", Value::Bool(false)),
        ("padProgress", Value::F64(0.0)),
        ("padPlayMode", Value::U32(0)),
        ("padTriggerMode", Value::U32(1)),
        ("padTriggerType", Value::U32(0)),
        (
            "padTriggerControl",
            Value::U32(u32::try_from(pad_idx).unwrap_or(0)),
        ),
        ("padTriggerOn", Value::U32(127)),
        ("padTriggerOff", Value::U32(0)),
        ("padTriggerSend", Value::U32(2)),
        ("padTriggerChannel", Value::U32(1)),
        ("padTriggerCustom", Value::Bool(false)),
        ("padIsInternal", Value::Bool(false)),
        ("padEffectInput", Value::U32(0)),
        ("padEffectTriggerMode", Value::U32(0)),
        ("padEnvStart", Value::F64(0.0)),
        ("padEnvStop", Value::F64(1.0)),
        ("padEnvFadeIn", Value::F64(0.0)),
        ("padEnvFadeOut", Value::F64(0.0)),
        ("padMixerMode", Value::U32(0)),
        ("padMixerTriggerMode", Value::U32(0)),
        ("padMixerFadeInSeconds", Value::F64(0.0)),
        ("padMixerFadeOutSeconds", Value::F64(0.0)),
        ("padMixerFadeExcludeHost", Value::Bool(false)),
        ("padMixerCensorCustom", Value::Bool(false)),
        ("padMixerCensorFilePath", Value::String(String::new())),
        ("padSIPCallSlot", Value::U32(0)),
        ("padSIPPhoneBookEntry", Value::U32(0)),
        ("padSIPFlashState", Value::U32(0)),
        ("padSIPQdLock", Value::Bool(false)),
        ("padRCVSyncPadType", Value::U32(0)),
        (
            "padProgressRequestSignal",
            Value::Combined(vec![Value::F64(0.0), Value::Bool(false)]),
        ),
    ];

    for (prop_name, value) in &props {
        if let Err(e) =
            conn.send_property_update(indices.clone(), (*prop_name).into(), value.clone())
        {
            warn!("failed to send {prop_name}: {e}");
        }
        std::thread::sleep(PROPERTY_UPDATE_DELAY);
    }

    info!("pad node created with {} properties", props.len());
    Ok(())
}

const MAX_AUDIO_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Validates that the path points to a supported audio file.
///
/// # Errors
/// Returns an error string if the file is missing, empty, too large, or not WAV/MP3.
pub fn validate_audio_file(path: &str) -> Result<String, String> {
    let p = std::path::Path::new(path);
    let meta = p
        .symlink_metadata()
        .map_err(|_| format!("file not found: {path}"))?;
    if meta.file_type().is_symlink() {
        return Err("symlinks are not supported".into());
    }
    if !meta.is_file() {
        return Err("not a regular file".into());
    }
    let size = meta.len();
    if size == 0 {
        return Err("file is empty".into());
    }
    if size > MAX_AUDIO_FILE_SIZE {
        return Err(format!("file too large ({size} bytes, max 100 MB)"));
    }
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext != "wav" && ext != "mp3" {
        return Err("only WAV and MP3 files are supported".into());
    }
    Ok(ext)
}

#[must_use]
pub fn device_pad_path(pad_idx: usize, extension: &str) -> String {
    format!(
        "/Application/emmc-data/pads/{}/sound.{extension}",
        pad_idx + 1
    )
}

/// Deletes a pad child node from the device.
///
/// # Errors
/// Returns an error if sending the child-removed packet fails.
pub fn delete_pad(
    conn: &DeviceConnection,
    soundpads_idx: usize,
    child_index: usize,
) -> rcp2_protocol::Result<()> {
    use rcp2_protocol::packet::child_removed::ChildRemovedPacket;

    info!("deleting pad at SOUNDPADS[{soundpads_idx}] child {child_index}");
    let packet = ChildRemovedPacket {
        path: vec![soundpads_idx],
        child_index,
    };
    conn.send_packet(Box::new(packet))
}

/// Activates file transfer mode on the device.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn activate_transfer_mode(conn: &DeviceConnection, mode: u32) -> rcp2_protocol::Result<()> {
    info!("activating transfer mode {mode}");
    conn.send_property_update(
        vec![SYSTEM_IDX],
        "transferModeType".into(),
        Value::U32(mode),
    )
}

/// Deactivates file transfer mode on the device.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn deactivate_transfer_mode(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("deactivating transfer mode");
    conn.send_property_update(vec![SYSTEM_IDX], "transferModeType".into(), Value::U32(0))
}

/// Triggers a remount of the pad storage on the device.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn remount_pad_storage(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("sending remountPadStorage");
    conn.send_property_update(
        vec![SYSTEM_IDX],
        "remountPadStorage".into(),
        Value::Bool(true),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_pad_path_formats_correctly() {
        assert_eq!(
            device_pad_path(0, "wav"),
            "/Application/emmc-data/pads/1/sound.wav"
        );
        assert_eq!(
            device_pad_path(7, "mp3"),
            "/Application/emmc-data/pads/8/sound.mp3"
        );
    }

    #[test]
    fn validate_audio_file_not_found() {
        let result = validate_audio_file("/tmp/rcp2_nonexistent_file.wav");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn validate_audio_file_wrong_extension() {
        let dir = std::env::temp_dir().join("rcp2_test_validate");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test.txt");
        std::fs::write(&path, b"data").ok();

        let result = validate_audio_file(&path.to_string_lossy());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("WAV and MP3"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validate_audio_file_valid_wav() {
        let dir = std::env::temp_dir().join("rcp2_test_validate_wav");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test.wav");
        std::fs::write(&path, b"RIFF data").ok();

        let result = validate_audio_file(&path.to_string_lossy());
        assert_eq!(result, Ok("wav".into()));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validate_audio_file_uppercase_extension() {
        let dir = std::env::temp_dir().join("rcp2_test_validate_upper");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test.WAV");
        std::fs::write(&path, b"RIFF data").ok();

        let result = validate_audio_file(&path.to_string_lossy());
        assert_eq!(result, Ok("wav".into()));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validate_audio_file_empty() {
        let dir = std::env::temp_dir().join("rcp2_test_validate_empty");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("empty.wav");
        std::fs::write(&path, b"").ok();

        let result = validate_audio_file(&path.to_string_lossy());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
