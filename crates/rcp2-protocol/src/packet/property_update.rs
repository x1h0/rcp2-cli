use crate::packet::{PacketParse, PacketSerialize};
use crate::types::Value;
use crate::types::c_string::{parse_c_string, write_c_string};
use nom::IResult;
use nom::bytes::streaming::tag;
use nom::error::{Error, ErrorKind};
use nom::number::streaming::{le_u8, le_u16};

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyUpdatePacket {
    pub indices: Vec<usize>,
    pub name: String,
    pub value: Value,
}

impl PacketParse for PropertyUpdatePacket {
    fn from_bytes(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag([0x01].as_ref())(input)?;
        let (input, _indices_count_length) = tag([0x01].as_ref())(input)?;
        let (input, indices_count) = le_u8(input)?;
        let (input, indices) = parse_indices(input, indices_count as usize)?;
        let (input, name) = parse_c_string(input)?;
        let (input, value) = Value::parse(input)?;
        Ok((
            input,
            Self {
                indices,
                name,
                value,
            },
        ))
    }
}

impl PacketSerialize for PropertyUpdatePacket {
    fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        let mut bytes = vec![0x01];
        bytes.push(0x01);
        bytes.push(
            u8::try_from(self.indices.len())
                .map_err(|_| crate::Error::Parse("too many indices".into()))?,
        );
        write_indices(&mut bytes, &self.indices)?;
        write_c_string(&mut bytes, &self.name);
        self.value.write(&mut bytes)?;
        Ok(bytes)
    }
}

fn parse_indices(input: &[u8], count: usize) -> IResult<&[u8], Vec<usize>> {
    let mut indices = Vec::with_capacity(count);
    let mut remaining = input;
    for _ in 0..count {
        let (input, index_len) = le_u8(remaining)?;
        let (input, index) = match index_len {
            0x00 => (input, 0),
            0x01 => le_u8(input).map(|(i, v)| (i, v as usize))?,
            0x02 => le_u16(input).map(|(i, v)| (i, v as usize))?,
            _ => return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify))),
        };
        remaining = input;
        indices.push(index);
    }
    Ok((remaining, indices))
}

fn write_indices(stream: &mut Vec<u8>, indices: &[usize]) -> crate::Result<()> {
    for &index in indices {
        if index == 0 {
            stream.push(0x00);
        } else if let Ok(val) = u8::try_from(index) {
            stream.push(0x01);
            stream.push(val);
        } else if let Ok(val) = u16::try_from(index) {
            stream.push(0x02);
            stream.extend_from_slice(&val.to_le_bytes());
        } else {
            return Err(crate::Error::Protocol(format!(
                "index value {index} too large"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    fn roundtrip(packet: &PropertyUpdatePacket) {
        let bytes = packet.to_bytes().unwrap();
        let (remaining, parsed) = PropertyUpdatePacket::from_bytes(&bytes).unwrap();
        assert!(remaining.is_empty(), "trailing bytes: {remaining:02x?}");
        assert_eq!(&parsed, packet);
    }

    #[test]
    fn roundtrip_simple() {
        roundtrip(&PropertyUpdatePacket {
            indices: vec![0],
            name: "volume".into(),
            value: Value::U32(80),
        });
    }

    #[test]
    fn roundtrip_nested_indices() {
        roundtrip(&PropertyUpdatePacket {
            indices: vec![3, 0, 7],
            name: "enabled".into(),
            value: Value::Bool(true),
        });
    }

    #[test]
    fn roundtrip_zero_index() {
        roundtrip(&PropertyUpdatePacket {
            indices: vec![0, 0],
            name: "x".into(),
            value: Value::U32(0),
        });
    }

    #[test]
    fn roundtrip_large_index() {
        roundtrip(&PropertyUpdatePacket {
            indices: vec![300],
            name: "big".into(),
            value: Value::U32(1),
        });
    }

    #[test]
    fn roundtrip_string_value() {
        roundtrip(&PropertyUpdatePacket {
            indices: vec![1],
            name: "label".into(),
            value: Value::String("Sound FX".into()),
        });
    }

    #[test]
    fn roundtrip_empty_indices() {
        roundtrip(&PropertyUpdatePacket {
            indices: vec![],
            name: "root_prop".into(),
            value: Value::F64(1.23),
        });
    }
}
