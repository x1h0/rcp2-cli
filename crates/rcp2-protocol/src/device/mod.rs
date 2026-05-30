pub mod connection;
pub mod state;

pub use connection::{DeviceConnection, DeviceEvent};
pub use state::DeviceState;

pub const PHYSICAL_INTERFACE_IDX: usize = 0;
pub const PADBUTTON_OFFSET: usize = 35;
