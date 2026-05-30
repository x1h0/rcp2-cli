use crate::transport::Transport;
use hidapi::{HidApi, HidDevice};
use log::{debug, trace};

const REPORT_ID_SEND: u8 = 0x03;
const REPORT_ID_RECV: u8 = 0x04;
const REPORT_SIZE: usize = 256;

pub const VENDOR_ID: u16 = 0x19f7;
pub const PRODUCT_IDS_MAIN: &[u16] = &[0x0030, 0x0037, 0x0072, 0x0078, 0x0094, 0x0092];
pub const PRODUCT_IDS_SECONDARY: &[u16] = &[0x0026];
pub const HID_INTERFACE: i32 = 9;

pub const FRAME_PAYLOAD_SIZE: usize = REPORT_SIZE - 1;

pub struct HidTransport {
    device: HidDevice,
}

impl HidTransport {
    /// Opens a HID transport to the device.
    ///
    /// # Errors
    /// Returns an error if no compatible device is found or it cannot be opened.
    pub fn open(hid_api: &HidApi) -> crate::Result<Self> {
        let device = open_device(hid_api)?;
        Ok(HidTransport { device })
    }

    /// Opens a pair of HID transports for separate RX/TX channels.
    ///
    /// # Errors
    /// Returns an error if no compatible device is found or it cannot be opened.
    pub fn open_pair(hid_api: &HidApi) -> crate::Result<(Self, Self)> {
        let rx = open_device(hid_api)?;
        let tx = open_device(hid_api)?;
        Ok((HidTransport { device: rx }, HidTransport { device: tx }))
    }

    #[must_use]
    pub fn enumerate(hid_api: &HidApi) -> Vec<DeviceInfo> {
        let mut seen = std::collections::HashSet::new();
        hid_api
            .device_list()
            .filter(|d| d.vendor_id() == VENDOR_ID)
            .filter(|d| {
                PRODUCT_IDS_MAIN.contains(&d.product_id())
                    || PRODUCT_IDS_SECONDARY.contains(&d.product_id())
            })
            .filter(|d| seen.insert(d.product_id()))
            .map(|d| {
                let port = if PRODUCT_IDS_MAIN.contains(&d.product_id()) {
                    PortType::Main
                } else if PRODUCT_IDS_SECONDARY.contains(&d.product_id()) {
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
                }
            })
            .collect()
    }
}

fn open_device(hid_api: &HidApi) -> crate::Result<HidDevice> {
    let secondary = hid_api
        .device_list()
        .any(|d| d.vendor_id() == VENDOR_ID && PRODUCT_IDS_SECONDARY.contains(&d.product_id()));

    let device_info = hid_api
        .device_list()
        .find(|d| {
            d.vendor_id() == VENDOR_ID
                && PRODUCT_IDS_MAIN.contains(&d.product_id())
                && d.interface_number() == HID_INTERFACE
        })
        .ok_or_else(|| {
            if secondary {
                crate::Error::Transport(
                    "RodeCaster Pro II found on secondary USB port - \
                     please connect via the main USB-C port for configuration access"
                        .into(),
                )
            } else {
                crate::Error::Transport("no RodeCaster Pro II found on HID interface 9".into())
            }
        })?;

    debug!(
        "opening device: VID={:04x} PID={:04x} interface={}",
        device_info.vendor_id(),
        device_info.product_id(),
        device_info.interface_number()
    );

    let device = device_info.open_device(hid_api)?;
    Ok(device)
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
        buf[0] = REPORT_ID_RECV;
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortType {
    Main,
    Secondary,
    Unknown,
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let port_label = match self.port {
            PortType::Main => "main",
            PortType::Secondary => "secondary",
            PortType::Unknown => "unknown",
        };
        write!(
            f,
            "{} [{}] (VID={:04x} PID={:04x}{})",
            self.product.as_deref().unwrap_or("RodeCaster Pro II"),
            port_label,
            self.vendor_id,
            self.product_id,
            self.serial
                .as_ref()
                .map(|s| format!(" S/N={s}"))
                .unwrap_or_default()
        )
    }
}
