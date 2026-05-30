use crate::packet::PacketSerialize;
use crate::types::c_string::write_c_string;

pub struct ChildAddedPacket {
    pub path: Vec<usize>,
    pub insert_index: usize,
    pub node_name: String,
}

impl PacketSerialize for ChildAddedPacket {
    fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        let mut bytes = vec![0x03];

        write_compressed_int(
            &mut bytes,
            i64::try_from(self.path.len())
                .map_err(|_| crate::Error::Parse("path length overflow".into()))?,
        );

        for &idx in &self.path {
            write_compressed_int(
                &mut bytes,
                i64::try_from(idx)
                    .map_err(|_| crate::Error::Parse("path index overflow".into()))?,
            );
        }

        write_compressed_int(
            &mut bytes,
            i64::try_from(self.insert_index)
                .map_err(|_| crate::Error::Parse("insert index overflow".into()))?,
        );

        write_c_string(&mut bytes, &self.node_name);

        bytes.push(0x00);

        bytes.push(0x00);

        Ok(bytes)
    }
}

pub(crate) fn write_compressed_int(buf: &mut Vec<u8>, value: i64) {
    let sign = u8::from(value < 0);
    let abs_val = value.unsigned_abs();

    if abs_val == 0 {
        buf.push(0x00);
        return;
    }

    let num_bytes = if abs_val <= 0xFF {
        1
    } else if abs_val <= 0xFFFF {
        2
    } else if abs_val <= 0xFF_FFFF {
        3
    } else if abs_val <= 0xFFFF_FFFF {
        4
    } else {
        5
    };

    buf.push(num_bytes | (sign << 7));
    for i in 0..num_bytes {
        buf.push(((abs_val >> (i * 8)) & 0xFF) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::PacketSerialize;

    #[test]
    fn serializes_simple_node() {
        let pkt = ChildAddedPacket {
            path: vec![0],
            insert_index: 5,
            node_name: "PAD".into(),
        };
        let bytes = pkt.to_bytes().unwrap();

        assert_eq!(bytes[0], 0x03);

        assert_eq!(&bytes[1..3], &[0x01, 0x01]);

        assert_eq!(bytes[3], 0x00);

        assert_eq!(&bytes[4..6], &[0x01, 0x05]);

        assert_eq!(&bytes[6..10], b"PAD\0");

        assert_eq!(&bytes[10..12], &[0x00, 0x00]);

        assert_eq!(bytes.len(), 12);
    }

    #[test]
    fn serializes_empty_path() {
        let pkt = ChildAddedPacket {
            path: vec![],
            insert_index: 0,
            node_name: "X".into(),
        };
        let bytes = pkt.to_bytes().unwrap();

        assert_eq!(bytes[0], 0x03);

        assert_eq!(bytes[1], 0x00);

        assert_eq!(bytes[2], 0x00);

        assert_eq!(&bytes[3..5], b"X\0");

        assert_eq!(&bytes[5..7], &[0x00, 0x00]);

        assert_eq!(bytes.len(), 7);
    }

    #[test]
    fn serializes_nested_path() {
        let pkt = ChildAddedPacket {
            path: vec![13, 2],
            insert_index: 10,
            node_name: "TEST".into(),
        };
        let bytes = pkt.to_bytes().unwrap();

        assert_eq!(bytes[0], 0x03);

        assert_eq!(&bytes[1..3], &[0x01, 0x02]);

        assert_eq!(&bytes[3..5], &[0x01, 0x0D]);

        assert_eq!(&bytes[5..7], &[0x01, 0x02]);

        assert_eq!(&bytes[7..9], &[0x01, 0x0A]);

        assert_eq!(&bytes[9..14], b"TEST\0");

        assert_eq!(&bytes[14..16], &[0x00, 0x00]);

        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn compressed_int_zero() {
        let mut buf = Vec::new();
        write_compressed_int(&mut buf, 0);
        assert_eq!(buf, vec![0x00]);
    }

    #[test]
    fn compressed_int_small() {
        let mut buf = Vec::new();
        write_compressed_int(&mut buf, 5);
        assert_eq!(buf, vec![0x01, 0x05]);
    }

    #[test]
    fn compressed_int_large() {
        let mut buf = Vec::new();
        write_compressed_int(&mut buf, 256);
        assert_eq!(buf, vec![0x02, 0x00, 0x01]);
    }

    #[test]
    fn compressed_int_negative() {
        let mut buf = Vec::new();
        write_compressed_int(&mut buf, -1);
        assert_eq!(buf, vec![0x81, 0x01]);
    }
}
