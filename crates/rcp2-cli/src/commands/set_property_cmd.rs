use super::Context;
use rcp2_protocol::types::Value;

pub fn set_property(
    ctx: &Context,
    indices_str: &str,
    name: &str,
    value_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if ctx.offline {
        println!("set-property requires a connected device");
        return Ok(());
    }

    let indices: Vec<usize> = indices_str
        .split(',')
        .map(|s| s.trim().parse())
        .collect::<Result<_, _>>()?;

    let value = parse_value(value_str)?;

    if ctx.dry_run {
        println!("dry-run: would set [{indices_str}] {name} = {value:?}");
        return Ok(());
    }

    let conn = super::open_connection(ctx)?;
    conn.wait_for_state()?;

    println!("setting [{indices_str}] {name} = {value:?}");
    conn.send_property_update(indices, name.into(), value)?;

    conn.flush()?;
    println!("done");

    Ok(())
}

fn parse_value(s: &str) -> Result<Value, Box<dyn std::error::Error>> {
    if let Some(v) = s.strip_prefix("u32:") {
        Ok(Value::U32(v.parse()?))
    } else if let Some(v) = s.strip_prefix("bool:") {
        Ok(Value::Bool(v == "true" || v == "1"))
    } else if let Some(v) = s.strip_prefix("f64:") {
        Ok(Value::F64(v.parse()?))
    } else if let Some(v) = s.strip_prefix("str:") {
        Ok(Value::String(v.into()))
    } else {
        Err(
            format!("invalid value format: {s} (use u32:N, bool:true/false, f64:N, str:text)")
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_value_u32() {
        let result = parse_value("u32:42").unwrap();
        assert_eq!(result, Value::U32(42));
    }

    #[test]
    fn parse_value_bool_true() {
        let result = parse_value("bool:true").unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn parse_value_bool_false() {
        let result = parse_value("bool:false").unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn parse_value_bool_one() {
        let result = parse_value("bool:1").unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn parse_value_f64() {
        let result = parse_value("f64:2.72").unwrap();
        assert_eq!(result, Value::F64(2.72));
    }

    #[test]
    fn parse_value_string() {
        let result = parse_value("str:hello world").unwrap();
        assert_eq!(result, Value::String("hello world".into()));
    }

    #[test]
    fn parse_value_negative_u32() {
        let result = parse_value("u32:-12");
        assert!(result.is_err());
    }

    #[test]
    fn parse_value_negative_f64() {
        let result = parse_value("f64:-12.5").unwrap();
        assert_eq!(result, Value::F64(-12.5));
    }

    #[test]
    fn parse_value_no_prefix() {
        let result = parse_value("42");
        assert!(result.is_err());
    }
}
