pub mod child_added;
pub mod child_removed;
pub mod device_report;
pub mod handshake;
pub mod property_update;

pub use device_report::DeviceReportPacket;
pub use property_update::PropertyUpdatePacket;

use nom::IResult;

pub trait PacketParse {
    /// Parses a packet from a byte slice.
    ///
    /// # Errors
    /// Returns a parse error if the bytes do not represent a valid packet.
    fn from_bytes(bytes: &[u8]) -> IResult<&[u8], Self>
    where
        Self: Sized;
}

pub trait PacketSerialize {
    /// Serializes this packet into bytes.
    ///
    /// # Errors
    /// Returns an error if the packet data cannot be encoded.
    fn to_bytes(&self) -> crate::Result<Vec<u8>>;
}
