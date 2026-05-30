pub mod connection;
pub mod model;
pub mod state;

pub use connection::{DeviceConnection, DeviceEvent};
pub use model::{DeviceModel, DeviceProfile};
pub use state::DeviceState;

pub const PHYSICAL_INTERFACE_IDX: usize = 0;
