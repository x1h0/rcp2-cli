use log::{info, warn};
use rcp2_protocol::device::{DeviceConnection, DeviceProfile, PHYSICAL_INTERFACE_IDX};
use rcp2_protocol::packet::child_added::ChildAddedPacket;
use rcp2_protocol::types::Value;
use std::collections::HashMap;
use std::time::Duration;

pub const DEFAULT_PRESS: Duration = Duration::from_millis(50);
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
    tap_pad_for(conn, pad_position, profile, DEFAULT_PRESS)
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
    press_pad(conn, pad_position, profile)?;
    std::thread::sleep(hold);
    release_pad(conn, pad_position, profile)
}

/// Presses (and holds) a SMART pad button. Pair with [`release_pad`].
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn press_pad(
    conn: &DeviceConnection,
    pad_position: usize,
    profile: &DeviceProfile,
) -> rcp2_protocol::Result<()> {
    let button_idx = profile.padbutton_offset + pad_position;
    conn.send_property_update(
        vec![PHYSICAL_INTERFACE_IDX, button_idx],
        "padButtonPressed".into(),
        Value::Bool(true),
    )
}

/// Releases a previously pressed SMART pad button.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn release_pad(
    conn: &DeviceConnection,
    pad_position: usize,
    profile: &DeviceProfile,
) -> rcp2_protocol::Result<()> {
    let button_idx = profile.padbutton_offset + pad_position;
    conn.send_property_update(
        vec![PHYSICAL_INTERFACE_IDX, button_idx],
        "padButtonPressed".into(),
        Value::Bool(false),
    )
}

/// Syncs the selected bank index to the device.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn sync_bank(conn: &DeviceConnection, bank: usize) -> rcp2_protocol::Result<()> {
    let gui_idx = conn.state().root_child_index("GUI")?;
    let value = Value::U32(u32::try_from(bank).unwrap_or(0));
    conn.send_property_update(vec![gui_idx], "selectedBank".into(), value.clone())?;
    if let Err(e) = conn.state().set_property(&[gui_idx], "selectedBank", value) {
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

    let device_path = pad_dir_file_path(pad_idx, filename);

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

fn clone_props_for_slot<S: std::hash::BuildHasher>(
    props: &HashMap<String, Value, S>,
    pad_idx: usize,
    device_path: &str,
) -> Vec<(String, Value)> {
    let idx_value = Value::U32(u32::try_from(pad_idx).unwrap_or(0));
    let custom_trigger = matches!(props.get("padTriggerCustom"), Some(Value::Bool(true)));

    let mut out = Vec::with_capacity(props.len() + 3);
    out.push(("padIdx".to_string(), idx_value.clone()));
    out.push((
        "padFilePath".to_string(),
        Value::String(device_path.to_string()),
    ));
    if !custom_trigger {
        out.push(("padTriggerControl".to_string(), idx_value));
    }
    for (name, value) in props {
        if name == "padIdx" || name == "padFilePath" {
            continue;
        }
        if name == "padTriggerControl" && !custom_trigger {
            continue;
        }
        out.push((name.clone(), value.clone()));
    }
    out
}

/// Creates a new PAD child node on the device by replaying an existing property
/// map, overriding `padIdx` and `padFilePath` for the target slot.
///
/// # Errors
/// Returns an error if sending the child-added packet fails.
pub fn create_pad_node_from_props<S: std::hash::BuildHasher>(
    conn: &DeviceConnection,
    soundpads_idx: usize,
    insert_at: usize,
    pad_idx: usize,
    device_path: &str,
    props: &HashMap<String, Value, S>,
) -> rcp2_protocol::Result<()> {
    info!(
        "cloning PAD node at SOUNDPADS[{soundpads_idx}] position {insert_at} for padIdx {pad_idx} ({} props)",
        props.len()
    );

    let packet = ChildAddedPacket {
        path: vec![soundpads_idx],
        insert_index: insert_at,
        node_name: "PAD".into(),
    };
    conn.send_packet(Box::new(packet))?;

    std::thread::sleep(NODE_CREATION_DELAY);

    let indices = vec![soundpads_idx, insert_at];
    for (name, value) in clone_props_for_slot(props, pad_idx, device_path) {
        if let Err(e) = conn.send_property_update(indices.clone(), name.clone(), value) {
            warn!("failed to send {name}: {e}");
        }
        std::thread::sleep(PROPERTY_UPDATE_DELAY);
    }

    info!("pad node cloned with {} properties", props.len());
    Ok(())
}

#[must_use]
pub fn pad_dir_file_path(pad_idx: usize, filename: &str) -> String {
    format!("/Application/emmc-data/pads/{}/{}", pad_idx + 1, filename)
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
    let system_idx = conn.state().root_child_index("SYSTEM")?;
    conn.send_property_update(
        vec![system_idx],
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
    let system_idx = conn.state().root_child_index("SYSTEM")?;
    conn.send_property_update(vec![system_idx], "transferModeType".into(), Value::U32(0))
}

/// Triggers a remount of the pad storage on the device.
///
/// # Errors
/// Returns an error if sending the property update fails.
pub fn remount_pad_storage(conn: &DeviceConnection) -> rcp2_protocol::Result<()> {
    info!("sending remountPadStorage");
    let system_idx = conn.state().root_child_index("SYSTEM")?;
    conn.send_property_update(
        vec![system_idx],
        "remountPadStorage".into(),
        Value::Bool(true),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_dir_file_path_formats_correctly() {
        assert_eq!(
            pad_dir_file_path(0, "sound.wav"),
            "/Application/emmc-data/pads/1/sound.wav"
        );
        assert_eq!(
            pad_dir_file_path(9, "sound.mp3"),
            "/Application/emmc-data/pads/10/sound.mp3"
        );
    }

    #[test]
    fn clone_props_overrides_idx_and_path() {
        let mut props = HashMap::new();
        props.insert("padIdx".to_string(), Value::U32(3));
        props.insert(
            "padFilePath".to_string(),
            Value::String("/Application/emmc-data/pads/4/sound.wav".into()),
        );
        props.insert("padName".to_string(), Value::String("Horn".into()));
        props.insert("padGain".to_string(), Value::F64(-6.0));
        props.insert("padTriggerControl".to_string(), Value::U32(3));
        props.insert("padTriggerCustom".to_string(), Value::Bool(false));

        let entries = clone_props_for_slot(&props, 10, "/Application/emmc-data/pads/11/sound.wav");

        assert_eq!(entries[0], ("padIdx".to_string(), Value::U32(10)));
        assert!(
            entries
                .iter()
                .any(|(n, v)| n == "padTriggerControl" && *v == Value::U32(10)),
            "non-custom trigger control must retarget to the new slot index"
        );
        assert_eq!(
            entries
                .iter()
                .filter(|(n, _)| n == "padTriggerControl")
                .count(),
            1
        );
        assert_eq!(
            entries[1],
            (
                "padFilePath".to_string(),
                Value::String("/Application/emmc-data/pads/11/sound.wav".into())
            )
        );
        assert_eq!(entries.iter().filter(|(n, _)| n == "padIdx").count(), 1);
        assert_eq!(
            entries.iter().filter(|(n, _)| n == "padFilePath").count(),
            1
        );
        assert!(
            entries
                .iter()
                .any(|(n, v)| n == "padName" && *v == Value::String("Horn".into()))
        );
        assert!(
            entries
                .iter()
                .any(|(n, v)| n == "padGain" && *v == Value::F64(-6.0))
        );
        assert_eq!(entries.len(), props.len());
    }

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
