use crate::types::c_string::parse_c_string;
use crate::types::value::Value;
use nom::IResult;
use nom::error::{Error, ErrorKind};
use nom::number::streaming::{le_u8, le_u16};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Structured {
    pub name: String,
    pub properties: HashMap<String, Value>,
    pub children: Vec<Structured>,
}

impl Structured {
    /// Parses a structured node from the wire format.
    ///
    /// # Errors
    /// Returns a parse error if the bytes are malformed.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        Self::parse_at(input, 0)
    }

    fn parse_at(input: &[u8], depth: usize) -> IResult<&[u8], Self> {
        if depth > crate::types::value::MAX_DEPTH {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::TooLarge)));
        }
        let (input, name) = parse_c_string(input)?;
        let (input, kind) = le_u8(input)?;

        match kind {
            0x00 => {
                let (input, child_count_len) = le_u8(input)?;
                let (input, child_count) = match child_count_len {
                    0x00 => (input, 0usize),
                    0x01 => le_u8(input).map(|(i, v)| (i, v as usize))?,
                    0x02 => le_u16(input).map(|(i, v)| (i, v as usize))?,
                    _ => return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify))),
                };

                let (input, children) = Self::parse_children(input, child_count, depth)?;
                Ok((
                    input,
                    Structured {
                        name,
                        properties: HashMap::new(),
                        children,
                    },
                ))
            }
            0x01 => {
                let (input, properties) = Self::parse_properties(input, depth)?;
                let (input, close_tag) = le_u8(input)?;

                match close_tag {
                    0x00 => Ok((
                        input,
                        Structured {
                            name,
                            properties,
                            children: vec![],
                        },
                    )),
                    0x01 => {
                        let (input, child_count) = le_u8(input)?;
                        let (input, children) =
                            Self::parse_children(input, child_count as usize, depth)?;
                        Ok((
                            input,
                            Structured {
                                name,
                                properties,
                                children,
                            },
                        ))
                    }
                    _ => Err(nom::Err::Error(Error::new(input, ErrorKind::Verify))),
                }
            }
            _ => Err(nom::Err::Error(Error::new(input, ErrorKind::Verify))),
        }
    }

    fn parse_children(input: &[u8], count: usize, depth: usize) -> IResult<&[u8], Vec<Structured>> {
        let mut children = Vec::new();
        let mut remaining = input;
        for _ in 0..count {
            let (input, child) = Structured::parse_at(remaining, depth + 1)?;
            children.push(child);
            remaining = input;
        }
        Ok((remaining, children))
    }

    fn parse_properties(input: &[u8], depth: usize) -> IResult<&[u8], HashMap<String, Value>> {
        let (input, count) = le_u8(input)?;
        let mut properties = HashMap::with_capacity(count as usize);
        let mut remaining = input;
        for _ in 0..count {
            let (input, name) = parse_c_string(remaining)?;
            let (input, value) = Value::parse_at(input, depth + 1)?;
            properties.insert(name, value);
            remaining = input;
        }
        Ok((remaining, properties))
    }

    #[must_use]
    pub fn get_property(&self, indices: &[usize], property_name: &str) -> Option<&Value> {
        if indices.is_empty() {
            self.properties.get(property_name)
        } else {
            self.children
                .get(indices[0])
                .and_then(|child| child.get_property(&indices[1..], property_name))
        }
    }

    /// Sets a property value at the given child path.
    ///
    /// # Errors
    /// Returns an error if the path is invalid, the property is missing, or types mismatch.
    pub fn set_property(
        &mut self,
        indices: &[usize],
        property_name: &str,
        value: Value,
    ) -> crate::Result<()> {
        if indices.is_empty() {
            if let Some(existing) = self.properties.get(property_name) {
                if std::mem::discriminant(existing) != std::mem::discriminant(&value) {
                    return Err(crate::Error::State(format!(
                        "type mismatch for property '{property_name}': existing {existing:?}, new {value:?}"
                    )));
                }
                self.properties.insert(property_name.to_string(), value);
                Ok(())
            } else {
                Err(crate::Error::State(format!(
                    "property '{property_name}' does not exist"
                )))
            }
        } else {
            let idx = indices[0];
            let child = self
                .children
                .get_mut(idx)
                .ok_or_else(|| crate::Error::State(format!("child index {idx} out of bounds")))?;
            child.set_property(&indices[1..], property_name, value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_object(name: &str, props: &[(&str, Value)]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(name.as_bytes());
        buf.push(0x00); // null terminator
        buf.push(0x01); // kind = object
        buf.push(u8::try_from(props.len()).expect("test has too many properties")); // property count
        for (pname, pval) in props {
            buf.extend_from_slice(pname.as_bytes());
            buf.push(0x00);
            pval.write(&mut buf)
                .expect("failed to write test property value");
        }
        buf.push(0x00); // close tag = no children
        buf
    }

    fn build_collection(name: &str, children_data: Vec<Vec<u8>>) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(name.as_bytes());
        buf.push(0x00);
        buf.push(0x00); // kind = collection
        if children_data.is_empty() {
            buf.push(0x00); // child_count_len = 0 (empty)
        } else {
            buf.push(0x01); // child_count_len = u8
            buf.push(u8::try_from(children_data.len()).expect("test has too many children"));
            for child in children_data {
                buf.extend_from_slice(&child);
            }
        }
        buf
    }

    #[test]
    fn deeply_nested_input_is_rejected_not_panicking() {
        let mut data = Vec::new();
        for _ in 0..(super::super::value::MAX_DEPTH + 50) {
            data.extend_from_slice(&[0x00, 0x00, 0x01, 0x01]);
        }
        data.extend_from_slice(&[0x00, 0x00, 0x00]);
        assert!(Structured::parse(&data).is_err());
    }

    #[test]
    fn parse_simple_object() {
        let data = build_object("myObj", &[("enabled", Value::Bool(true))]);
        let (remaining, result) = Structured::parse(&data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(result.name, "myObj");
        assert_eq!(result.properties.get("enabled"), Some(&Value::Bool(true)));
        assert!(result.children.is_empty());
    }

    #[test]
    fn parse_object_multiple_properties() {
        let data = build_object(
            "cfg",
            &[
                ("volume", Value::U32(80)),
                ("name", Value::String("pad1".into())),
            ],
        );
        let (_, result) = Structured::parse(&data).unwrap();
        assert_eq!(result.properties.len(), 2);
        assert_eq!(result.properties.get("volume"), Some(&Value::U32(80)));
        assert_eq!(
            result.properties.get("name"),
            Some(&Value::String("pad1".into()))
        );
    }

    #[test]
    fn parse_empty_collection() {
        let data = build_collection("empty", vec![]);
        let (remaining, result) = Structured::parse(&data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(result.name, "empty");
        assert!(result.properties.is_empty());
        assert!(result.children.is_empty());
    }

    #[test]
    fn parse_collection_with_children() {
        let child1 = build_object("a", &[("x", Value::U32(1))]);
        let child2 = build_object("b", &[("y", Value::U32(2))]);
        let data = build_collection("parent", vec![child1, child2]);

        let (remaining, result) = Structured::parse(&data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(result.name, "parent");
        assert_eq!(result.children.len(), 2);
        assert_eq!(result.children[0].name, "a");
        assert_eq!(result.children[1].name, "b");
    }

    #[test]
    fn set_property_direct() {
        let data = build_object("obj", &[("val", Value::U32(10))]);
        let (_, mut s) = Structured::parse(&data).unwrap();

        s.set_property(&[], "val", Value::U32(20)).unwrap();
        assert_eq!(s.properties.get("val"), Some(&Value::U32(20)));
    }

    #[test]
    fn set_property_type_mismatch() {
        let data = build_object("obj", &[("val", Value::U32(10))]);
        let (_, mut s) = Structured::parse(&data).unwrap();

        let result = s.set_property(&[], "val", Value::Bool(true));
        assert!(result.is_err());
    }

    #[test]
    fn set_property_nonexistent() {
        let data = build_object("obj", &[("val", Value::U32(10))]);
        let (_, mut s) = Structured::parse(&data).unwrap();

        let result = s.set_property(&[], "missing", Value::U32(1));
        assert!(result.is_err());
    }

    #[test]
    fn set_property_nested() {
        let child = build_object("child", &[("x", Value::U32(1))]);
        let data = build_collection("root", vec![child]);
        let (_, mut s) = Structured::parse(&data).unwrap();

        s.set_property(&[0], "x", Value::U32(99)).unwrap();
        assert_eq!(s.get_property(&[0], "x"), Some(&Value::U32(99)));
    }

    #[test]
    fn set_property_index_out_of_bounds() {
        let data = build_collection("root", vec![]);
        let (_, mut s) = Structured::parse(&data).unwrap();

        let result = s.set_property(&[5], "x", Value::U32(1));
        assert!(result.is_err());
    }

    #[test]
    fn get_property_nested() {
        let child = build_object("inner", &[("flag", Value::Bool(false))]);
        let data = build_collection("outer", vec![child]);
        let (_, s) = Structured::parse(&data).unwrap();

        assert_eq!(s.get_property(&[0], "flag"), Some(&Value::Bool(false)));
        assert_eq!(s.get_property(&[0], "nope"), None);
        assert_eq!(s.get_property(&[1], "flag"), None);
    }
}
