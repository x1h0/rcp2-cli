use super::Context;
use log::info;
use rcp2_protocol::transport::Transport;
use rcp2_protocol::transport::hid::HidTransport;

pub fn send(ctx: &Context, hex: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = parse_hex(hex)?;

    if ctx.offline {
        println!("offline mode: would send {} bytes", bytes.len());
        println!("  {bytes:02x?}");
        return Ok(());
    }

    if ctx.dry_run {
        println!("dry-run: would send {} bytes", bytes.len());
        println!("  {bytes:02x?}");
        return Ok(());
    }

    let hid_api = hidapi::HidApi::new()?;
    let (mut transport, _model) = HidTransport::open(&hid_api)?;

    info!("sending {} bytes", bytes.len());
    transport.send(&bytes)?;
    println!("sent {} bytes", bytes.len());

    info!("waiting for response...");
    match transport.recv() {
        Ok(response) => {
            println!("response: {} bytes", response.len());
            println!("  {:02x?}", &response[..std::cmp::min(64, response.len())]);
        }
        Err(e) => {
            eprintln!("no response: {e}");
        }
    }

    Ok(())
}

fn parse_hex(hex: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let hex = hex.replace(' ', "").replace("0x", "").replace(',', "");
    if !hex.len().is_multiple_of(2) {
        return Err("hex string must have even length".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(std::convert::Into::into))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_valid() {
        let result = parse_hex("AD10A7B0").unwrap();
        assert_eq!(result, vec![0xAD, 0x10, 0xA7, 0xB0]);
    }

    #[test]
    fn parse_hex_lowercase() {
        let result = parse_hex("ad10a7b0").unwrap();
        assert_eq!(result, vec![0xAD, 0x10, 0xA7, 0xB0]);
    }

    #[test]
    fn parse_hex_with_prefix() {
        let result = parse_hex("0xAD0x10").unwrap();
        assert_eq!(result, vec![0xAD, 0x10]);
    }

    #[test]
    fn parse_hex_odd_length() {
        let result = parse_hex("ABC");
        assert!(result.is_err());
    }

    #[test]
    fn parse_hex_invalid_chars() {
        let result = parse_hex("GGGG");
        assert!(result.is_err());
    }

    #[test]
    fn parse_hex_empty() {
        let result = parse_hex("").unwrap();
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn parse_hex_with_spaces() {
        let result = parse_hex("AD 10 A7 B0").unwrap();
        assert_eq!(result, vec![0xAD, 0x10, 0xA7, 0xB0]);
    }

    #[test]
    fn parse_hex_with_commas() {
        let result = parse_hex("AD,10,A7,B0").unwrap();
        assert_eq!(result, vec![0xAD, 0x10, 0xA7, 0xB0]);
    }
}
