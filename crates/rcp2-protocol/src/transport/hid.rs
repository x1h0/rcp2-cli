use crate::device::model::DeviceModel;
use crate::transport::Transport;
use hidapi::{HidApi, HidDevice};
use log::{debug, trace};

const REPORT_ID_SEND: u8 = 0x03;
const REPORT_SIZE: usize = 256;

pub const VENDOR_ID: u16 = 0x19f7;

pub const PRODUCT_IDS_PRO_II: &[u16] = &[0x0030, 0x0037, 0x0072, 0x0078, 0x0094, 0x0092];
pub const PRODUCT_IDS_PRO_II_SECONDARY: &[u16] = &[0x0026];

pub const PRODUCT_IDS_DUO: &[u16] = &[0x0050, 0x0079, 0x0093, 0x0095];
pub const PRODUCT_IDS_DUO_SECONDARY: &[u16] = &[0x004e];

pub const HID_INTERFACE: i32 = 9;

pub const FRAME_PAYLOAD_SIZE: usize = REPORT_SIZE - 1;

fn is_known_main(pid: u16) -> bool {
    PRODUCT_IDS_PRO_II.contains(&pid) || PRODUCT_IDS_DUO.contains(&pid)
}

fn is_known_secondary(pid: u16) -> bool {
    PRODUCT_IDS_PRO_II_SECONDARY.contains(&pid) || PRODUCT_IDS_DUO_SECONDARY.contains(&pid)
}

fn is_known(pid: u16) -> bool {
    is_known_main(pid) || is_known_secondary(pid)
}

pub struct HidTransport {
    device: HidDevice,
}

impl HidTransport {
    /// Opens a HID transport to the device.
    ///
    /// # Errors
    /// Returns an error if no compatible device is found or it cannot be opened.
    pub fn open(hid_api: &HidApi) -> crate::Result<(Self, DeviceModel)> {
        let (device, model) = open_device(hid_api)?;
        Ok((HidTransport { device }, model))
    }

    /// Opens a pair of HID transports for separate RX/TX channels.
    ///
    /// # Errors
    /// Returns an error if no compatible device is found or it cannot be opened.
    pub fn open_pair(hid_api: &HidApi) -> crate::Result<((Self, Self), DeviceModel)> {
        let (rx, model) = open_device(hid_api)?;
        let (tx, _) = open_device(hid_api)?;
        Ok((
            (HidTransport { device: rx }, HidTransport { device: tx }),
            model,
        ))
    }

    #[must_use]
    pub fn enumerate(hid_api: &HidApi) -> Vec<DeviceInfo> {
        let mut seen = std::collections::HashSet::new();
        hid_api
            .device_list()
            .filter(|d| d.vendor_id() == VENDOR_ID)
            .filter(|d| is_known(d.product_id()))
            .filter(|d| seen.insert(d.product_id()))
            .map(|d| {
                let port = if is_known_main(d.product_id()) {
                    PortType::Main
                } else if is_known_secondary(d.product_id()) {
                    PortType::Secondary
                } else {
                    PortType::Unknown
                };
                DeviceInfo {
                    vendor_id: d.vendor_id(),
                    product_id: d.product_id(),
                    serial: d.serial_number().map(String::from),
                    product: d.product_string().map(String::from),
                    port,
                    model: DeviceModel::from_product_id(d.product_id()),
                }
            })
            .collect()
    }
}

fn open_device(hid_api: &HidApi) -> crate::Result<(HidDevice, DeviceModel)> {
    let secondary = hid_api
        .device_list()
        .any(|d| d.vendor_id() == VENDOR_ID && is_known_secondary(d.product_id()));

    let device_info = hid_api
        .device_list()
        .find(|d| {
            d.vendor_id() == VENDOR_ID
                && is_known_main(d.product_id())
                && d.interface_number() == HID_INTERFACE
        })
        .ok_or_else(|| {
            if secondary {
                crate::Error::Transport(
                    "RØDECaster device found on secondary USB port - \
                     please connect via the main USB-C port for configuration access"
                        .into(),
                )
            } else {
                crate::Error::Transport(
                    "no supported RØDECaster device found on HID interface".into(),
                )
            }
        })?;

    let model = DeviceModel::from_product_id(device_info.product_id()).ok_or_else(|| {
        crate::Error::Transport(format!(
            "unknown product ID {:04x}",
            device_info.product_id()
        ))
    })?;

    debug!(
        "opening {model} device: VID={:04x} PID={:04x} interface={}",
        device_info.vendor_id(),
        device_info.product_id(),
        device_info.interface_number()
    );

    let device = device_info.open_device(hid_api)?;
    Ok((device, model))
}

impl Transport for HidTransport {
    fn send(&mut self, data: &[u8]) -> crate::Result<()> {
        let mut report = vec![REPORT_ID_SEND];
        report.extend_from_slice(data);
        if report.len() < REPORT_SIZE {
            report.resize(REPORT_SIZE, 0x00);
        }
        trace!(
            "HID TX: {:02x?}",
            &report[..std::cmp::min(32, report.len())]
        );
        self.device
            .send_output_report(&report)
            .map_err(|e| crate::Error::Transport(format!("HID send failed: {e}")))?;
        Ok(())
    }

    fn recv(&mut self) -> crate::Result<Vec<u8>> {
        let mut buf = [0u8; REPORT_SIZE];
        let n = self
            .device
            .read_timeout(&mut buf, 100)
            .map_err(|e| crate::Error::Transport(format!("HID recv failed: {e}")))?;
        if n > 0 {
            trace!("HID RX: {:02x?}", &buf[..std::cmp::min(32, buf.len())]);
            Ok(buf[1..].to_vec())
        } else {
            Err(crate::Error::Timeout)
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial: Option<String>,
    pub product: Option<String>,
    pub port: PortType,
    pub model: Option<DeviceModel>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortType {
    Main,
    Secondary,
    Unknown,
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.model.map_or_else(
            || self.product.as_deref().unwrap_or("RØDECaster"),
            |m| m.profile().display_name,
        );
        let port_label = match self.port {
            PortType::Main => "main",
            PortType::Secondary => "secondary",
            PortType::Unknown => "unknown",
        };
        write!(
            f,
            "{name} [{port_label}] (VID={:04x} PID={:04x}{})",
            self.vendor_id,
            self.product_id,
            self.serial
                .as_ref()
                .map(|s| format!(" S/N={s}"))
                .unwrap_or_default()
        )
    }
}
