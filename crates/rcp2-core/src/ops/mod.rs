pub mod audio;
pub mod download;
pub mod fader;
pub mod move_pad;
pub mod pad;
pub mod transfer;
pub mod upload;

pub mod gui;
pub mod network;
pub mod recorder;
pub mod system;

pub const SYSTEM_IDX: usize = 13;
pub const NETWORK_IDX: usize = 14;
pub const GUI_IDX: usize = 5;
pub const RECORDER_IDX: usize = 2;
pub const DEVICE_PAD_PREFIX: &str = "/Application/emmc-data/pads/";
pub const TRANSFER_MODE_EMMC: u32 = 2;
pub const TRANSFER_MODE_SD: u32 = 1;

#[must_use]
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        let whole = bytes / KB;
        let frac = (bytes % KB) * 10 / KB;
        format!("{whole}.{frac} KB")
    } else if bytes < GB {
        let whole = bytes / MB;
        let frac = (bytes % MB) * 10 / MB;
        format!("{whole}.{frac} MB")
    } else {
        let whole = bytes / GB;
        let frac = (bytes % GB) * 10 / GB;
        format!("{whole}.{frac} GB")
    }
}
