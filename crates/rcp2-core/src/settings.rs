use crate::DeviceViewModel;

pub use crate::settings_labels::{LANGUAGE_CODES, TIMEZONE_COUNT, language_label, timezone_label};

#[derive(Clone, Copy)]
pub enum SettingsCategory {
    Network,
    Bluetooth,
    Midi,
    Language,
    Clock,
    DeviceInfo,
    Analytics,
}

pub const CATEGORIES: [SettingsCategory; 7] = [
    SettingsCategory::Network,
    SettingsCategory::Bluetooth,
    SettingsCategory::Midi,
    SettingsCategory::Language,
    SettingsCategory::Clock,
    SettingsCategory::DeviceInfo,
    SettingsCategory::Analytics,
];

pub const SETTINGS_CATEGORY_COUNT: usize = CATEGORIES.len();

#[derive(Clone, Copy)]
pub enum Field {
    WifiEnabled,
    BtDisconnect,
    SerialReveal,
    UpdateCheck,
    MidiControl,
    AnalyticsUsage,
    AnalyticsResource,
    Language,
    Clock24h,
    ClockDst,
    ClockOnHome,
    Timezone,
    Apply,
}

#[derive(Clone, Copy)]
pub enum SettingsNode {
    System,
    Network,
}

#[derive(Clone, Copy)]
pub enum SettingsStep {
    Activate,
    Prev,
    Next,
}

#[derive(Clone, Copy)]
pub enum DraftField {
    Lang,
    Timezone,
}

#[derive(Clone, Copy)]
pub enum FieldRole {
    Live,
    Stage(DraftField),
    Apply,
    BtDisconnect,
    ToggleSerial,
    CheckUpdate,
}

#[must_use]
pub fn category_fields(category: SettingsCategory, bt_connected: bool) -> &'static [Field] {
    match category {
        SettingsCategory::Network => &[Field::WifiEnabled],
        SettingsCategory::Bluetooth if bt_connected => &[Field::BtDisconnect],
        SettingsCategory::DeviceInfo => &[Field::SerialReveal, Field::UpdateCheck],
        SettingsCategory::Midi => &[Field::MidiControl],
        SettingsCategory::Analytics => &[Field::AnalyticsUsage, Field::AnalyticsResource],
        SettingsCategory::Language => &[Field::Language, Field::Apply],
        SettingsCategory::Clock => &[
            Field::Clock24h,
            Field::ClockDst,
            Field::ClockOnHome,
            Field::Timezone,
            Field::Apply,
        ],
        SettingsCategory::Bluetooth => &[],
    }
}

#[must_use]
pub fn bt_connected(vm: &DeviceViewModel) -> bool {
    !vm.network.bluetooth.connected.is_empty()
}

impl SettingsCategory {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            SettingsCategory::Network => "Network",
            SettingsCategory::Bluetooth => "Bluetooth",
            SettingsCategory::Midi => "MIDI Control",
            SettingsCategory::Language => "Language",
            SettingsCategory::Clock => "Date & Clock",
            SettingsCategory::DeviceInfo => "Device Info",
            SettingsCategory::Analytics => "Analytics",
        }
    }
}

impl Field {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Field::WifiEnabled => "Enabled",
            Field::BtDisconnect => "Disconnect",
            Field::SerialReveal => "Serial",
            Field::UpdateCheck => "Check",
            Field::MidiControl => "MIDI",
            Field::AnalyticsUsage => "Usage",
            Field::AnalyticsResource => "Resource",
            Field::Language => "Language",
            Field::Clock24h => "24h",
            Field::ClockDst => "DST",
            Field::ClockOnHome => "Show on home",
            Field::Timezone => "Timezone",
            Field::Apply => "Apply",
        }
    }

    #[must_use]
    pub fn role(self) -> FieldRole {
        match self {
            Field::WifiEnabled
            | Field::MidiControl
            | Field::AnalyticsUsage
            | Field::AnalyticsResource
            | Field::Clock24h
            | Field::ClockDst
            | Field::ClockOnHome => FieldRole::Live,
            Field::Language => FieldRole::Stage(DraftField::Lang),
            Field::Timezone => FieldRole::Stage(DraftField::Timezone),
            Field::Apply => FieldRole::Apply,
            Field::BtDisconnect => FieldRole::BtDisconnect,
            Field::SerialReveal => FieldRole::ToggleSerial,
            Field::UpdateCheck => FieldRole::CheckUpdate,
        }
    }
}

fn step_index(idx: usize, len: usize, step: SettingsStep) -> usize {
    match step {
        SettingsStep::Prev => (idx + len - 1) % len,
        SettingsStep::Activate | SettingsStep::Next => (idx + 1) % len,
    }
}

#[must_use]
pub fn settings_item_count(vm: &DeviceViewModel, category_index: usize) -> usize {
    CATEGORIES
        .get(category_index)
        .map_or(0, |c| category_fields(*c, bt_connected(vm)).len())
}

#[must_use]
pub fn settings_field_role(
    vm: &DeviceViewModel,
    category_index: usize,
    item: usize,
) -> Option<FieldRole> {
    let category = *CATEGORIES.get(category_index)?;
    let field = *category_fields(category, bt_connected(vm)).get(item)?;
    Some(field.role())
}

#[must_use]
pub fn settings_live_toggle(
    vm: &DeviceViewModel,
    category_index: usize,
    item: usize,
    step: SettingsStep,
) -> Option<(SettingsNode, &'static str, bool)> {
    if !matches!(step, SettingsStep::Activate) {
        return None;
    }
    let category = *CATEGORIES.get(category_index)?;
    let field = *category_fields(category, bt_connected(vm)).get(item)?;
    let system = &vm.system;
    let (node, name, current) = match field {
        Field::WifiEnabled => (SettingsNode::Network, "wifi", vm.network.wifi.enabled),
        Field::MidiControl => (
            SettingsNode::System,
            "systemMidiControl",
            system.midi_control,
        ),
        Field::AnalyticsUsage => (
            SettingsNode::System,
            "shareAnom",
            system.analytics.share_anom,
        ),
        Field::AnalyticsResource => (
            SettingsNode::System,
            "shareResource",
            system.analytics.share_resource,
        ),
        Field::Clock24h => (
            SettingsNode::System,
            "systemDateTime24h",
            system.clock.format_24h,
        ),
        Field::ClockDst => (
            SettingsNode::System,
            "systemDateTimeDaylightSavings",
            system.clock.daylight_savings,
        ),
        Field::ClockOnHome => (
            SettingsNode::System,
            "systemDateTimeOnHome",
            system.clock.on_home,
        ),
        _ => return None,
    };
    Some((node, name, !current))
}

#[must_use]
pub fn category_draft_fields(category_index: usize) -> &'static [DraftField] {
    match CATEGORIES.get(category_index) {
        Some(SettingsCategory::Language) => &[DraftField::Lang],
        Some(SettingsCategory::Clock) => &[DraftField::Timezone],
        _ => &[],
    }
}

#[must_use]
pub fn settings_category_is_language(category_index: usize) -> bool {
    matches!(
        CATEGORIES.get(category_index),
        Some(SettingsCategory::Language)
    )
}

#[must_use]
pub fn cycle_language(index: usize, step: SettingsStep) -> usize {
    step_index(index, LANGUAGE_CODES.len(), step)
}

#[must_use]
pub fn cycle_timezone(index: usize, step: SettingsStep) -> usize {
    step_index(index, TIMEZONE_COUNT, step)
}

#[must_use]
pub fn language_index(code: &str) -> usize {
    LANGUAGE_CODES.iter().position(|c| *c == code).unwrap_or(0)
}

#[must_use]
pub fn language_code(index: usize) -> &'static str {
    LANGUAGE_CODES.get(index).copied().unwrap_or("en")
}
