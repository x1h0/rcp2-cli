pub mod ops;
mod pad;

pub use pad::{
    BankView, PAD_COLS, PAD_ROWS, PADS_PER_BANK, PHYSICAL_ORDER, PadColor, PadInfo, PadType,
};

use rcp2_protocol::types::{Structured, Value};

pub const MAX_BANKS: usize = 8;

pub struct DeviceViewModel {
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
}

pub const PHYSICAL_FADERS: usize = 6;
pub const VIRTUAL_FADERS: usize = 3;

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
    fn from_u32(v: u32) -> Self {
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
    pub inserted: bool,
    pub mounted: bool,
    pub rec_destination: bool,
    pub capacity_bytes: u64,
    pub free_bytes: u64,
}

impl StorageInfo {
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.inserted || self.capacity_bytes > 0
    }
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
}

#[derive(Debug, Clone, Default)]
pub struct NetworkInfo {
    pub wifi_enabled: bool,
    pub wifi_ssid: String,
    pub wired: bool,
    pub bt_connected: String,
}

impl DeviceViewModel {
    #[must_use]
    pub fn from_state(state: &Structured) -> Self {
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

        let faders = extract_faders(state);
        let pots = extract_pots(state);
        let channels = extract_channels(state);
        let recorder = extract_recorder(state);
        let storage = extract_storage(state);
        let show = extract_show(state);
        let system = extract_system(state);
        let network = extract_network(state);

        DeviceViewModel {
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
        }
    }

    #[must_use]
    pub fn bank_view(&self, bank: usize) -> BankView {
        BankView::from_pads(&self.pads, bank)
    }

    #[must_use]
    pub fn current_bank_view(&self) -> BankView {
        self.bank_view(self.selected_bank)
    }

    #[must_use]
    pub fn bank_count(&self) -> usize {
        MAX_BANKS
    }

    pub fn has_storage(&self) -> bool {
        self.storage.iter().any(StorageInfo::is_available)
    }

    pub fn refresh(&mut self, state: &Structured) {
        *self = Self::from_state(state);
    }
}

fn extract_faders(state: &Structured) -> Vec<FaderInfo> {
    let channels = extract_channels(state);
    let mix_nodes: Vec<&Structured> = state.children.iter().filter(|c| c.name == "MIX").collect();
    let total = (PHYSICAL_FADERS + VIRTUAL_FADERS).min(channels.len());

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

fn extract_pots(state: &Structured) -> Vec<u32> {
    find_node(state, "PHYSICALINTERFACE")
        .map(|pi| {
            pi.children
                .iter()
                .filter(|c| c.name == "POT")
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
                    let (capacity, free) = parse_storage_state(&state_str);
                    StorageInfo {
                        name: get_string(c, "storageVolumeName"),
                        inserted: get_bool(c, "storageVolumeInserted"),
                        mounted: get_bool(c, "storageVolumeMounted"),
                        rec_destination: get_bool(c, "storageVolumeRecDestination"),
                        capacity_bytes: capacity,
                        free_bytes: free,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_storage_state(state: &str) -> (u64, u64) {
    let parts: Vec<&str> = state.split('|').collect();
    if parts.len() >= 2 {
        let capacity = parts[0].parse().unwrap_or(0);
        let free = parts[1].parse().unwrap_or(0);
        (capacity, free)
    } else {
        (0, 0)
    }
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
    find_node(state, "SYSTEM")
        .map(|s| SystemInfo {
            firmware: get_string(s, "systemFirmwareVersion"),
            serial: get_string(s, "systemSerialNumber"),
            name: get_string(s, "systemName"),
        })
        .unwrap_or_default()
}

fn extract_network(state: &Structured) -> NetworkInfo {
    find_node(state, "NETWORK")
        .map(|n| NetworkInfo {
            wifi_enabled: get_bool(n, "wifi"),
            wifi_ssid: get_string(n, "wifiSSID"),
            wired: get_bool(n, "wiredConnected"),
            bt_connected: get_string(n, "btConnectedAddress"),
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
