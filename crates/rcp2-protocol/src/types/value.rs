use crate::types::c_string::{parse_c_string, write_c_string};
use log::warn;
use nom::IResult;
use nom::bytes::streaming::take;
use nom::error::{Error, ErrorKind};
use nom::number::streaming::{le_f64, le_u8, le_u16, le_u32};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    U32(u32),
    Bool(bool),
    F64(f64),
    String(String),
    Double(f64),
    Combined(Vec<Value>),
    Unknown(Vec<u8>),
}

impl Value {
    /// Parses a typed value from the wire format.
    ///
    /// # Errors
    /// Returns a parse error if the type tag or length is invalid.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, data_length_type) = le_u8(input)?;
        let (input, data_length) = match data_length_type {
            0x01 => le_u8(input).map(|(i, v)| (i, v as usize))?,
            0x02 => le_u16(input).map(|(i, v)| (i, v as usize))?,
            _ => return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify))),
        };

        let (input, data_type) = le_u8(input)?;
        match data_type {
            0x01 => {
                if data_length - 1 != 4 {
                    return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)));
                }
                let (input, value) = le_u32(input)?;
                Ok((input, Value::U32(value)))
            }
            0x02 => Ok((input, Value::Bool(true))),
            0x03 => Ok((input, Value::Bool(false))),
            0x04 => {
                if data_length - 1 != 8 {
                    return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)));
                }
                let (input, value) = le_f64(input)?;
                Ok((input, Value::F64(value)))
            }
            0x05 => {
                let (input, value) = parse_c_string(input)?;
                Ok((input, Value::String(value)))
            }
            0x06 => {
                if data_length - 1 != 8 {
                    return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)));
                }
                let (input, value) = le_f64(input)?;
                Ok((input, Value::Double(value)))
            }
            0x08 => {
                let mut remaining = data_length - 1;
                let mut values = vec![];
                let mut current = input;
                while remaining > 0 {
                    let (next, value) = Self::parse(current)?;
                    values.push(value);
                    let consumed = current.len().saturating_sub(next.len());
                    if consumed > remaining {
                        return Err(nom::Err::Error(Error::new(current, ErrorKind::Verify)));
                    }
                    remaining -= consumed;
                    current = next;
                }
                Ok((current, Value::Combined(values)))
            }
            _ => {
                warn!("unknown value type: 0x{data_type:02x}, length: {data_length}");
                let (input, value) = take(data_length - 1)(input)?;
                Ok((input, Value::Unknown(value.to_vec())))
            }
        }
    }

    /// Writes this value in wire format to the given buffer.
    ///
    /// # Errors
    /// Returns an error if the value is too large or is an unknown type.
    pub fn write(&self, stream: &mut Vec<u8>) -> crate::Result<()> {
        match self {
            Value::U32(value) => {
                stream.push(0x01);
                stream.push(0x05);
                stream.push(0x01);
                stream.extend_from_slice(&value.to_le_bytes());
            }
            Value::Bool(value) => {
                stream.push(0x01);
                stream.push(0x01);
                stream.push(if *value { 0x02 } else { 0x03 });
            }
            Value::F64(value) => {
                stream.push(0x01);
                stream.push(0x09);
                stream.push(0x04);
                stream.extend_from_slice(&value.to_le_bytes());
            }
            Value::String(value) => {
                let total_len = value.len() + 2;
                if let Ok(len) = u8::try_from(total_len) {
                    stream.push(0x01);
                    stream.push(len);
                } else if let Ok(len) = u16::try_from(total_len) {
                    stream.push(0x02);
                    stream.extend_from_slice(&len.to_le_bytes());
                } else {
                    return Err(crate::Error::Protocol("string too long".into()));
                }
                stream.push(0x05);
                write_c_string(stream, value);
            }
            Value::Double(value) => {
                stream.push(0x01);
                stream.push(0x09);
                stream.push(0x06);
                stream.extend_from_slice(&value.to_le_bytes());
            }
            Value::Combined(values) => {
                let mut body = Vec::new();
                for v in values {
                    v.write(&mut body)?;
                }
                let total_len = body.len() + 1;
                if let Ok(len) = u8::try_from(total_len) {
                    stream.push(0x01);
                    stream.push(len);
                } else if let Ok(len) = u16::try_from(total_len) {
                    stream.push(0x02);
                    stream.extend_from_slice(&len.to_le_bytes());
                } else {
                    return Err(crate::Error::Protocol("combined value too large".into()));
                }
                stream.push(0x08);
                stream.extend_from_slice(&body);
            }
            Value::Unknown(data) => {
                return Err(crate::Error::Protocol(format!(
                    "cannot write unknown value type ({} bytes)",
                    data.len()
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(value: &Value) {
        let mut buf = Vec::new();
        value.write(&mut buf).unwrap();
        let (remaining, parsed) = Value::parse(&buf).unwrap();
        assert!(
            remaining.is_empty(),
            "trailing bytes after parse: {remaining:02x?}"
        );
        assert_eq!(&parsed, value);
    }

    #[test]
    fn roundtrip_u32() {
        roundtrip(&Value::U32(0));
        roundtrip(&Value::U32(42));
        roundtrip(&Value::U32(u32::MAX));
    }

    #[test]
    fn roundtrip_bool() {
        roundtrip(&Value::Bool(true));
        roundtrip(&Value::Bool(false));
    }

    #[test]
    fn roundtrip_f64() {
        roundtrip(&Value::F64(0.0));
        roundtrip(&Value::F64(1.23456));
        roundtrip(&Value::F64(-1.0));
    }

    #[test]
    fn roundtrip_string() {
        roundtrip(&Value::String(String::new()));
        roundtrip(&Value::String("hello".into()));
        roundtrip(&Value::String("RØDECaster".into()));
    }

    #[test]
    fn roundtrip_double() {
        roundtrip(&Value::Double(0.0));
        roundtrip(&Value::Double(99.99));
    }

    #[test]
    fn roundtrip_combined() {
        roundtrip(&Value::Combined(vec![
            Value::U32(1),
            Value::Bool(true),
            Value::String("test".into()),
        ]));
    }

    #[test]
    fn roundtrip_nested_combined() {
        roundtrip(&Value::Combined(vec![
            Value::Combined(vec![Value::U32(1), Value::U32(2)]),
            Value::Bool(false),
        ]));
    }

    #[test]
    fn write_unknown_fails() {
        let val = Value::Unknown(vec![0x01, 0x02]);
        let mut buf = Vec::new();
        assert!(val.write(&mut buf).is_err());
    }

    #[test]
    fn parse_unknown_type() {
        let data = [0x01, 0x03, 0xFF, 0xAA, 0xBB];
        let (remaining, parsed) = Value::parse(&data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(parsed, Value::Unknown(vec![0xAA, 0xBB]));
    }

    #[test]
    fn parse_with_trailing_data() {
        let mut buf = Vec::new();
        Value::U32(42).write(&mut buf).unwrap();
        buf.extend_from_slice(&[0xDE, 0xAD]);
        let (remaining, parsed) = Value::parse(&buf).unwrap();
        assert_eq!(remaining, &[0xDE, 0xAD]);
        assert_eq!(parsed, Value::U32(42));
    }
}
