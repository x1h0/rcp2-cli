use nom::IResult;
use nom::bytes::streaming::{tag, take_till};

/// Parses a null-terminated C string from the input bytes.
///
/// # Errors
/// Returns a parse error if no null terminator is found.
pub fn parse_c_string(input: &[u8]) -> IResult<&[u8], String> {
    let (input, bytes) = take_till(|b| b == 0)(input)?;
    let (input, _) = tag([0u8].as_ref())(input)?;
    let string = String::from_utf8_lossy(bytes).into_owned();
    Ok((input, string))
}

pub fn write_c_string(stream: &mut Vec<u8>, string: &str) {
    stream.extend_from_slice(string.as_bytes());
    stream.push(0x00);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ascii() {
        let mut buf = Vec::new();
        write_c_string(&mut buf, "hello");
        let (remaining, parsed) = parse_c_string(&buf).unwrap();
        assert_eq!(parsed, "hello");
        assert!(remaining.is_empty());
    }

    #[test]
    fn roundtrip_empty() {
        let mut buf = Vec::new();
        write_c_string(&mut buf, "");
        assert_eq!(buf, vec![0x00]);
        let (remaining, parsed) = parse_c_string(&buf).unwrap();
        assert_eq!(parsed, "");
        assert!(remaining.is_empty());
    }

    #[test]
    fn parse_with_trailing_data() {
        let data = b"test\x00\xff\xaa";
        let (remaining, parsed) = parse_c_string(data).unwrap();
        assert_eq!(parsed, "test");
        assert_eq!(remaining, &[0xff, 0xaa]);
    }

    #[test]
    fn parse_missing_null_terminator() {
        let data = b"no terminator";
        assert!(parse_c_string(data).is_err());
    }
}
