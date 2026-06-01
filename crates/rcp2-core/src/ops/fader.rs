use log::info;
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::{Structured, Value};

#[must_use]
pub fn channel_indices(state: &Structured) -> Vec<usize> {
    state
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.name == "CHANNEL")
        .map(|(i, _)| i)
        .collect()
}

/// # Errors
/// Returns an error if the property update cannot be sent.
pub fn set_mute(
    conn: &DeviceConnection,
    channel_index: usize,
    mute: bool,
) -> rcp2_protocol::Result<()> {
    info!("setting channel {channel_index} mute = {mute}");
    conn.send_property_update(
        vec![channel_index],
        "channelOutputMute".into(),
        Value::Bool(mute),
    )
}

/// # Errors
/// Returns an error if the property update cannot be sent.
pub fn set_listen(
    conn: &DeviceConnection,
    channel_index: usize,
    listen: bool,
) -> rcp2_protocol::Result<()> {
    info!("setting channel {channel_index} listen = {listen}");
    conn.send_property_update(
        vec![channel_index],
        "channelCueEnable".into(),
        Value::Bool(listen),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn node(name: &str) -> Structured {
        Structured {
            name: name.into(),
            properties: HashMap::new(),
            children: Vec::new(),
        }
    }

    #[test]
    fn channel_indices_finds_real_positions() {
        let root = Structured {
            name: "ROOT".into(),
            properties: HashMap::new(),
            children: vec![
                node("RECORDER"),
                node("CHANNEL"),
                node("CHANNEL"),
                node("MIX"),
                node("CHANNEL"),
            ],
        };
        assert_eq!(channel_indices(&root), vec![1, 2, 4]);
    }

    #[test]
    fn channel_indices_empty_when_none() {
        let root = node("ROOT");
        assert!(channel_indices(&root).is_empty());
    }
}
