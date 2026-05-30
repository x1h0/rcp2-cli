pub mod hid;
pub mod mock;

pub trait Transport: Send {
    /// Sends a data frame to the device.
    ///
    /// # Errors
    /// Returns an error if the underlying I/O write fails.
    fn send(&mut self, data: &[u8]) -> crate::Result<()>;

    /// Receives a data frame from the device.
    ///
    /// # Errors
    /// Returns an error if the underlying I/O read fails or times out.
    fn recv(&mut self) -> crate::Result<Vec<u8>>;
}
