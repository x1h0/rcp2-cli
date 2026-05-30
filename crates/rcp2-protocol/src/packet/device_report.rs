use crate::packet::{PacketParse, PacketSerialize};
use crate::types::Structured;
use nom::IResult;
use nom::bytes::streaming::tag;

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceReportPacket {
    pub report: Structured,
}

impl PacketParse for DeviceReportPacket {
    fn from_bytes(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag([0x02].as_ref())(input)?;
        let (input, report) = Structured::parse(input)?;
        Ok((input, DeviceReportPacket { report }))
    }
}

impl PacketSerialize for DeviceReportPacket {
    fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        Err(crate::Error::Protocol(
            "DeviceReportPacket serialization not implemented".into(),
        ))
    }
}
