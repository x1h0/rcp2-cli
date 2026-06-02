use crate::packet::PacketSerialize;

pub struct ChildRemovedPacket {
    pub path: Vec<usize>,
    pub child_index: usize,
}

impl PacketSerialize for ChildRemovedPacket {
    fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        let mut bytes = vec![0x04];

        super::child_added::write_compressed_int(
            &mut bytes,
            i64::try_from(self.path.len())
                .map_err(|_| crate::Error::Protocol("path length overflow".into()))?,
        );
        for &idx in &self.path {
            super::child_added::write_compressed_int(
                &mut bytes,
                i64::try_from(idx)
                    .map_err(|_| crate::Error::Protocol("path index overflow".into()))?,
            );
        }
        super::child_added::write_compressed_int(
            &mut bytes,
            i64::try_from(self.child_index)
                .map_err(|_| crate::Error::Protocol("child index overflow".into()))?,
        );

        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::PacketSerialize;

    #[test]
    fn serializes_simple_remove() {
        let pkt = ChildRemovedPacket {
            path: vec![0],
            child_index: 3,
        };
        let bytes = pkt.to_bytes().unwrap();

        assert_eq!(bytes[0], 0x04);

        assert_eq!(&bytes[1..3], &[0x01, 0x01]);

        assert_eq!(bytes[3], 0x00);

        assert_eq!(&bytes[4..6], &[0x01, 0x03]);

        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn serializes_nested_path() {
        let pkt = ChildRemovedPacket {
            path: vec![13, 2],
            child_index: 7,
        };
        let bytes = pkt.to_bytes().unwrap();

        assert_eq!(bytes[0], 0x04);

        assert_eq!(&bytes[1..3], &[0x01, 0x02]);

        assert_eq!(&bytes[3..5], &[0x01, 0x0D]);

        assert_eq!(&bytes[5..7], &[0x01, 0x02]);

        assert_eq!(&bytes[7..9], &[0x01, 0x07]);

        assert_eq!(bytes.len(), 9);
    }
}
