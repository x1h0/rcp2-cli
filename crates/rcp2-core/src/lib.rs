pub mod ops;
mod pad;

pub use pad::{BankView, PadColor, PadInfo, PadType};
pub use rcp2_protocol::device::{DeviceModel, DeviceProfile};

use rcp2_protocol::types::{Structured, Value};

pub struct DeviceViewModel {
    pub profile: &'static DeviceProfile,
    pub selected_bank: usize,
    pub soundpads_idx: Option<usize>,
    pub pads: Vec<PadInfo>,
    pub faders: Vec<FaderInfo>,
    pub pots: Vec<u32>,
    pub channels: Vec<ChannelInfo>,
    pub recorder: RecorderState,
    pub storage: Vec<StorageInfo>,
    pub show: ShowInfo,
    pub system: SystemInfo,
    pub network: NetworkInfo,
    pub build: BuildInfo,
}

#[derive(Debug, Clone, Default)]
pub struct FaderInfo {
    pub level: f64,
    pub mute: bool,
    pub cue: bool,
    pub configured: bool,
}

impl FaderInfo {
    #[must_use]
    pub fn percent(&self) -> f64 {
        self.level.clamp(0.0, 1.0)
    }
}

const MIXES_PER_CHANNEL: usize = 13;
const INVALID_INPUT_SOURCE: u32 = u32::MAX;

#[derive(Debug, Clone, Default)]
pub struct ChannelInfo {
    pub mute: bool,
    pub cue: bool,
    pub input_source: u32,
}

#[derive(Debug, Clone, Default)]
pub struct RecorderState {
    pub state: RecordingStatus,
    pub time_ms: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum RecordingStatus {
    #[default]
    Stopped,
    Recording,
    Paused,
}

impl RecordingStatus {
    #[must_use]
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => RecordingStatus::Paused,
            2 => RecordingStatus::Recording,
            _ => RecordingStatus::Stopped,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            RecordingStatus::Stopped => "STOP",
            RecordingStatus::Recording => "REC",
            RecordingStatus::Paused => "PAUSE",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StorageInfo {
    pub name: String,
    pub present: bool,
    pub removable: bool,
    pub rec_destination: bool,
    pub capacity_bytes: u64,
    pub free_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ShowInfo {
    pub name: String,
    pub icon: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SystemInfo {
    pub firmware: String,
    pub serial: String,
    pub name: String,
    pub language: String,
    pub midi_control: bool,
    pub clock: ClockInfo,
    pub analytics: AnalyticsInfo,
    pub update: UpdateInfo,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateInfo {
    pub checking: bool,
    pub available: bool,
    pub no_internet: bool,
    pub version: String,
}

#[derive(Debug, Clone, Default)]
pub struct ClockInfo {
    pub format_24h: bool,
    pub daylight_savings: bool,
    pub on_home: bool,
    pub timezone_index: u32,
}

#[derive(Debug, Clone, Default)]
pub struct AnalyticsInfo {
    pub share_anom: bool,
    pub share_resource: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BuildInfo {
    pub mixer_version: String,
    pub gui_version: String,
    pub callme_version: String,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkInfo {
    pub wired: WiredInfo,
    pub wifi: WifiInfo,
    pub bluetooth: BluetoothInfo,
}

#[derive(Debug, Clone, Default)]
pub struct WiredInfo {
    pub connected: bool,
    pub ip_address: String,
    pub gateway: String,
    pub subnet_mask: String,
    pub primary_dns: String,
    pub secondary_dns: String,
}

#[derive(Debug, Clone, Default)]
pub struct WifiInfo {
    pub enabled: bool,
    pub ssid: String,
    pub ip: String,
    pub dhcp: bool,
    pub static_ip_set: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BluetoothInfo {
    pub visible: bool,
    pub connected: String,
}

impl DeviceViewModel {
    #[must_use]
    pub fn from_state(state: &Structured, profile: &'static DeviceProfile) -> Self {
        let selected_bank = find_node(state, "GUI")
            .and_then(|gui| get_u32_prop(gui, "selectedBank"))
            .unwrap_or(0) as usize;

        let soundpads_idx = find_node_index(state, "SOUNDPADS");

        let pads = find_node(state, "SOUNDPADS")
            .map(|sp| {
                sp.children
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.properties.contains_key("padIdx"))
                    .map(|(i, n)| PadInfo::from_node_at(n, i))
                    .collect()
            })
            .unwrap_or_default();

        let faders = extract_faders(state, profile);
        let pots = extract_pots(state, profile);
        let channels = extract_channels(state);
        let recorder = extract_recorder(state);
        let storage = extract_storage(state);
        let show = extract_show(state);
        let system = extract_system(state);
        let network = extract_network(state);
        let build = extract_build(state);

        DeviceViewModel {
            profile,
            selected_bank,
            soundpads_idx,
            pads,
            faders,
            pots,
            channels,
            recorder,
            storage,
            show,
            system,
            network,
            build,
        }
    }

    #[must_use]
    pub fn bank_view(&self, bank: usize) -> BankView {
        BankView::from_pads(&self.pads, bank, self.profile)
    }

    #[must_use]
    pub fn current_bank_view(&self) -> BankView {
        self.bank_view(self.selected_bank)
    }

    #[must_use]
    pub fn bank_count(&self) -> usize {
        self.profile.max_banks
    }

    #[must_use]
    pub fn has_storage(&self) -> bool {
        self.sd_storage().is_some()
    }

    #[must_use]
    pub fn sd_storage(&self) -> Option<&StorageInfo> {
        self.storage.iter().find(|s| s.removable && s.present)
    }

    #[must_use]
    pub fn internal_storage(&self) -> Option<&StorageInfo> {
        self.storage.iter().find(|s| !s.removable && s.present)
    }

    pub fn refresh(&mut self, state: &Structured) {
        let profile = self.profile;
        *self = Self::from_state(state, profile);
    }
}

fn extract_faders(state: &Structured, profile: &DeviceProfile) -> Vec<FaderInfo> {
    let channels = extract_channels(state);
    let mix_nodes: Vec<&Structured> = state.children.iter().filter(|c| c.name == "MIX").collect();
    let total = (profile.physical_faders + profile.virtual_faders).min(channels.len());

    channels[..total]
        .iter()
        .map(|ch| {
            let configured = ch.input_source != INVALID_INPUT_SOURCE;
            let level = if configured {
                let mix_offset = (ch.input_source as usize).checked_mul(MIXES_PER_CHANNEL);
                mix_offset
                    .and_then(|i| mix_nodes.get(i))
                    .map_or(0.0, |n| parse_mix_level(n))
            } else {
                0.0
            };
            FaderInfo {
                level,
                mute: ch.mute,
                cue: ch.cue,
                configured,
            }
        })
        .collect()
}

fn parse_mix_level(node: &Structured) -> f64 {
    let raw = get_string(node, "mixLevelWithAnchor");
    raw.split('|')
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0)
}

// The firmware always exposes four POT nodes; only the first physical_pots are real knobs.
fn extract_pots(state: &Structured, profile: &DeviceProfile) -> Vec<u32> {
    find_node(state, "PHYSICALINTERFACE")
        .map(|pi| {
            pi.children
                .iter()
                .filter(|c| c.name == "POT")
                .take(profile.physical_pots)
                .map(|c| get_u32(c, "potLevel"))
                .collect()
        })
        .unwrap_or_default()
}

fn extract_channels(state: &Structured) -> Vec<ChannelInfo> {
    state
        .children
        .iter()
        .filter(|c| c.name == "CHANNEL")
        .map(|c| ChannelInfo {
            mute: get_bool(c, "channelOutputMute"),
            cue: get_bool(c, "channelCueEnable"),
            input_source: get_u32(c, "channelInputSource"),
        })
        .collect()
}

fn extract_recorder(state: &Structured) -> RecorderState {
    find_node(state, "RECORDER")
        .map(|r| RecorderState {
            state: RecordingStatus::from_u32(get_u32(r, "recordState")),
            time_ms: get_u32(r, "recordTimeMs"),
        })
        .unwrap_or_default()
}

fn extract_storage(state: &Structured) -> Vec<StorageInfo> {
    find_node(state, "SYSTEM")
        .map(|sys| {
            sys.children
                .iter()
                .filter(|c| c.name == "STORAGEVOLUME")
                .map(|c| {
                    let state_str = get_string(c, "storageVolumeState");
                    let (capacity, free, removable) = parse_storage_state(&state_str);
                    StorageInfo {
                        name: get_string(c, "storageVolumeName"),
                        present: capacity > 0,
                        removable,
                        rec_destination: get_bool(c, "storageVolumeRecDestination"),
                        capacity_bytes: capacity,
                        free_bytes: free,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_storage_state(state: &str) -> (u64, u64, bool) {
    let parts: Vec<&str> = state.split('|').collect();
    let capacity = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let free = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let removable = parts.get(2).is_some_and(|s| *s == "1");
    (capacity, free, removable)
}

fn extract_show(state: &Structured) -> ShowInfo {
    find_node(state, "CURRENTSHOW")
        .map(|s| ShowInfo {
            name: get_string(s, "currentShowName"),
            icon: get_u32(s, "currentShowIcon"),
        })
        .unwrap_or_default()
}

fn extract_system(state: &Structured) -> SystemInfo {
    let language = find_node(state, "GUI")
        .map(|g| get_string(g, "lang"))
        .unwrap_or_default();
    find_node(state, "SYSTEM")
        .map(|s| SystemInfo {
            firmware: get_string(s, "systemFirmwareVersion"),
            serial: get_string(s, "systemSerialNumber"),
            name: get_string(s, "systemName"),
            language,
            midi_control: get_bool(s, "systemMidiControl"),
            clock: ClockInfo {
                format_24h: get_bool(s, "systemDateTime24h"),
                daylight_savings: get_bool(s, "systemDateTimeDaylightSavings"),
                on_home: get_bool(s, "systemDateTimeOnHome"),
                timezone_index: get_u32(s, "systemDateTimezone"),
            },
            analytics: AnalyticsInfo {
                share_anom: get_bool(s, "shareAnom"),
                share_resource: get_bool(s, "shareResource"),
            },
            update: UpdateInfo {
                checking: get_bool(s, "updateChecking"),
                available: get_bool(s, "osUpdateAvailable") || get_bool(s, "appUpdateAvailable"),
                no_internet: get_bool(s, "updateNoInternet"),
                version: get_string(s, "updateVersion"),
            },
        })
        .unwrap_or_default()
}

fn extract_build(state: &Structured) -> BuildInfo {
    find_node(state, "BUILD")
        .map(|b| BuildInfo {
            mixer_version: get_string(b, "buildMixerVersion"),
            gui_version: get_string(b, "buildGuiVersion"),
            callme_version: get_string(b, "buildCallMeVersion"),
        })
        .unwrap_or_default()
}

fn extract_network(state: &Structured) -> NetworkInfo {
    find_node(state, "NETWORK")
        .map(|n| NetworkInfo {
            wired: WiredInfo {
                connected: get_bool(n, "wiredConnected"),
                ip_address: get_string(n, "ipAddress"),
                gateway: get_string(n, "gateway"),
                subnet_mask: get_string(n, "subnetMask"),
                primary_dns: get_string(n, "primaryDns"),
                secondary_dns: get_string(n, "secondaryDns"),
            },
            wifi: WifiInfo {
                enabled: get_bool(n, "wifi"),
                ssid: get_string(n, "wifiSSID"),
                ip: get_string(n, "wifiIpAddress"),
                dhcp: get_bool(n, "wifiDHCP"),
                static_ip_set: get_bool(n, "staticIpSet"),
            },
            bluetooth: BluetoothInfo {
                visible: get_bool(n, "btVisible"),
                connected: get_string(n, "btConnectedAddress"),
            },
        })
        .unwrap_or_default()
}

fn find_node<'a>(root: &'a Structured, name: &str) -> Option<&'a Structured> {
    root.children.iter().find(|c| c.name == name)
}

fn find_node_index(root: &Structured, name: &str) -> Option<usize> {
    root.children.iter().position(|c| c.name == name)
}

fn get_u32_prop(node: &Structured, name: &str) -> Option<u32> {
    match node.properties.get(name) {
        Some(Value::U32(v)) => Some(*v),
        _ => None,
    }
}

pub(crate) fn get_u32(node: &Structured, name: &str) -> u32 {
    match node.properties.get(name) {
        Some(Value::U32(v)) => *v,
        _ => 0,
    }
}

pub(crate) fn get_f64(node: &Structured, name: &str) -> f64 {
    match node.properties.get(name) {
        Some(Value::F64(v) | Value::Double(v)) => *v,
        _ => 0.0,
    }
}

pub(crate) fn get_bool(node: &Structured, name: &str) -> bool {
    match node.properties.get(name) {
        Some(Value::Bool(v)) => *v,
        _ => false,
    }
}

pub(crate) fn get_string(node: &Structured, name: &str) -> String {
    match node.properties.get(name) {
        Some(Value::String(v)) => v.clone(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_storage_state;

    #[test]
    fn parses_storage_volume_state() {
        // capacity|free|removable|... — real device strings: SD, eMMC, empty.
        assert_eq!(
            parse_storage_state("64084770816|63855788032|1|1|1"),
            (64_084_770_816, 63_855_788_032, true)
        );
        assert_eq!(
            parse_storage_state("5277638656|5191655424|0|1|1"),
            (5_277_638_656, 5_191_655_424, false)
        );
        assert_eq!(parse_storage_state("0|0|0|0|0"), (0, 0, false));
        assert_eq!(parse_storage_state(""), (0, 0, false));
    }
}
