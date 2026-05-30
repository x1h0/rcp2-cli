use crate::packet::{DeviceReportPacket, PacketParse, PacketSerialize, PropertyUpdatePacket};
use log::{debug, trace};
use nom::bytes::streaming::take;
use nom::number::streaming::le_u32;
use std::cmp::min;

#[derive(Debug, Clone, PartialEq)]
pub enum PacketResult {
    Unknown(Vec<u8>),
    PropertyUpdate(PropertyUpdatePacket),
    DeviceReport(DeviceReportPacket),
}

const MAX_PACKET_SIZE: usize = 1024 * 1024; // 1MB

/// Reads and reassembles a framed message from sequential frames.
///
/// # Errors
/// Returns an error if a frame read fails or the packet is malformed.
pub fn read_framed_message<F>(mut get_next_frame: F) -> crate::Result<PacketResult>
where
    F: FnMut() -> crate::Result<Vec<u8>>,
{
    let current_frame = get_next_frame()?;
    let (first_remaining, packet_length) = le_u32::<&[u8], ()>(current_frame.as_slice())
        .map_err(|e| crate::Error::Parse(format!("failed to read packet length: {e}")))?;

    let packet_len = usize::try_from(packet_length)
        .map_err(|_| crate::Error::Parse("packet length overflow".into()))?;

    if packet_len > MAX_PACKET_SIZE {
        return Err(crate::Error::Parse(format!(
            "packet too large: {packet_length} bytes (max {MAX_PACKET_SIZE})"
        )));
    }

    trace!("incoming framed message: {packet_length} bytes");

    let mut packet = Vec::with_capacity(packet_len);
    let mut remaining_bytes = packet_len;
    let mut current_frame = first_remaining.to_vec();

    while remaining_bytes > 0 {
        let to_read = min(remaining_bytes, current_frame.len());
        let (_, bytes) = take::<usize, &[u8], ()>(to_read)(current_frame.as_slice())
            .map_err(|e| crate::Error::Parse(format!("failed to read frame data: {e}")))?;

        remaining_bytes -= bytes.len();
        packet.extend_from_slice(bytes);

        if remaining_bytes > 0 {
            current_frame = get_next_frame()?;
        }
    }

    trace!(
        "reassembled packet: {} bytes, type: 0x{:02x}",
        packet.len(),
        packet.first().copied().unwrap_or(0)
    );

    match packet.first().copied() {
        Some(0x01) => {
            let result = PropertyUpdatePacket::from_bytes(&packet)
                .map(|(_, p)| p)
                .map_err(|e| crate::Error::Parse(format!("property update: {e}")))?;
            Ok(PacketResult::PropertyUpdate(result))
        }
        Some(0x02) => {
            let result = DeviceReportPacket::from_bytes(&packet)
                .map(|(_, p)| p)
                .map_err(|e| crate::Error::Parse(format!("device report: {e}")))?;
            Ok(PacketResult::DeviceReport(result))
        }
        other => {
            debug!("unknown packet type: {other:?}");
            Ok(PacketResult::Unknown(packet))
        }
    }
}

/// Serializes and writes a packet as framed chunks.
///
/// # Errors
/// Returns an error if serialization or a frame write fails.
pub fn write_framed_message(
    packet: &dyn PacketSerialize,
    frame_size: usize,
    mut write_frame: impl FnMut(&[u8]) -> crate::Result<()>,
) -> crate::Result<()> {
    let body = packet.to_bytes()?;
    let body_len = u32::try_from(body.len()).map_err(|_| {
        crate::Error::Protocol("packet body too large for u32 length prefix".into())
    })?;
    let length_prefix = body_len.to_le_bytes();

    let mut full_message = Vec::with_capacity(4 + body.len());
    full_message.extend_from_slice(&length_prefix);
    full_message.extend_from_slice(&body);

    trace!(
        "writing framed message: {} bytes in {} byte frames",
        full_message.len(),
        frame_size
    );

    for chunk in full_message.chunks(frame_size) {
        let mut frame = chunk.to_vec();
        if frame.len() < frame_size {
            frame.resize(frame_size, 0x00);
        }
        write_frame(&frame)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    #[test]
    fn test_read_framed_unknown() {
        let frames: Vec<Vec<u8>> = vec![
            vec![0x02, 0x00, 0x00, 0x00, 0xFF],
            vec![0xAA, 0x00, 0x00, 0x00, 0x00],
        ];
        let mut iter = frames.into_iter();

        let result = read_framed_message(|| {
            iter.next()
                .ok_or_else(|| crate::Error::Transport("no more frames".into()))
        })
        .unwrap();

        assert!(matches!(result, PacketResult::Unknown(_)));
        if let PacketResult::Unknown(data) = result {
            assert_eq!(data, vec![0xFF, 0xAA]);
        }
    }

    #[test]
    fn test_write_framed_roundtrip() {
        let packet = PropertyUpdatePacket {
            indices: vec![1, 2],
            name: "test".into(),
            value: Value::U32(42),
        };

        let mut captured: Vec<Vec<u8>> = Vec::new();
        write_framed_message(&packet, 10, |data| {
            captured.push(data.to_vec());
            Ok(())
        })
        .unwrap();

        assert_eq!(3, captured.len());
        assert_eq!(
            vec![
                vec![0x13, 0x00, 0x00, 0x00, 0x01, 0x01, 0x02, 0x01, 0x01, 0x01],
                vec![0x02, 0x74, 0x65, 0x73, 0x74, 0x00, 0x01, 0x05, 0x01, 0x2A],
                vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            ],
            captured
        );
    }
}
