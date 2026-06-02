use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use rcp2_core::SystemInfo;

use super::settings_labels::{LANGUAGE_CODES, TIMEZONE_COUNT, language_label, timezone_label};

const FIELD_LABEL_WIDTH: usize = 13;

#[derive(Clone, Copy)]
enum SettingsCategory {
    Network,
    Bluetooth,
    Midi,
    Language,
    Clock,
    DeviceInfo,
    Analytics,
}

const CATEGORIES: [SettingsCategory; 7] = [
    SettingsCategory::Network,
    SettingsCategory::Bluetooth,
    SettingsCategory::Midi,
    SettingsCategory::Language,
    SettingsCategory::Clock,
    SettingsCategory::DeviceInfo,
    SettingsCategory::Analytics,
];

pub(crate) const SETTINGS_CATEGORY_COUNT: usize = CATEGORIES.len();

#[derive(Clone, Copy)]
enum Field {
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
pub(crate) enum SettingsNode {
    System,
    Network,
}

#[derive(Clone, Copy)]
pub(crate) enum SettingsStep {
    Activate,
    Prev,
    Next,
}

#[derive(Clone, Copy)]
pub(crate) enum DraftField {
    Lang,
    Timezone,
}

#[derive(Clone, Copy)]
pub(crate) enum FieldRole {
    Live,
    Stage(DraftField),
    Apply,
    BtDisconnect,
    ToggleSerial,
    CheckUpdate,
}

fn category_fields(category: SettingsCategory, bt_connected: bool) -> &'static [Field] {
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

fn bt_connected(vm: &rcp2_core::DeviceViewModel) -> bool {
    !vm.network.bluetooth.connected.is_empty()
}

impl Field {
    fn label(self) -> &'static str {
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

    fn role(self) -> FieldRole {
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

pub(crate) fn settings_item_count(vm: &rcp2_core::DeviceViewModel, category_index: usize) -> usize {
    CATEGORIES
        .get(category_index)
        .map_or(0, |c| category_fields(*c, bt_connected(vm)).len())
}

pub(crate) fn settings_field_role(
    vm: &rcp2_core::DeviceViewModel,
    category_index: usize,
    item: usize,
) -> Option<FieldRole> {
    let category = *CATEGORIES.get(category_index)?;
    let field = *category_fields(category, bt_connected(vm)).get(item)?;
    Some(field.role())
}

pub(crate) fn settings_live_toggle(
    vm: &rcp2_core::DeviceViewModel,
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

pub(crate) fn category_draft_fields(category_index: usize) -> &'static [DraftField] {
    match CATEGORIES.get(category_index) {
        Some(SettingsCategory::Language) => &[DraftField::Lang],
        Some(SettingsCategory::Clock) => &[DraftField::Timezone],
        _ => &[],
    }
}

pub(crate) fn settings_category_is_language(category_index: usize) -> bool {
    matches!(
        CATEGORIES.get(category_index),
        Some(SettingsCategory::Language)
    )
}

pub(crate) fn cycle_language(index: usize, step: SettingsStep) -> usize {
    step_index(index, LANGUAGE_CODES.len(), step)
}

pub(crate) fn cycle_timezone(index: usize, step: SettingsStep) -> usize {
    step_index(index, TIMEZONE_COUNT, step)
}

pub(crate) fn language_index(code: &str) -> usize {
    LANGUAGE_CODES.iter().position(|c| *c == code).unwrap_or(0)
}

pub(crate) fn language_code(index: usize) -> &'static str {
    LANGUAGE_CODES.get(index).copied().unwrap_or("en")
}

impl SettingsCategory {
    fn label(self) -> &'static str {
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

pub(super) fn render_settings(frame: &mut Frame, area: Rect, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(30)])
        .split(area);
    render_category_list(frame, layout[0], app);
    render_category_detail(frame, layout[1], app);
}

fn render_category_list(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = CATEGORIES
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let selected = i == app.settings_selected;
            let cursor = if selected { "\u{25B8} " } else { "  " };
            let style = if selected {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(vec![
                Span::styled(cursor, Style::default().fg(Color::Cyan)),
                Span::styled(cat.label(), style),
            ])
        })
        .collect();

    let list = Paragraph::new(lines).block(
        Block::default()
            .title(" Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .padding(Padding::new(1, 1, 0, 0)),
    );
    frame.render_widget(list, area);
}

fn render_category_detail(frame: &mut Frame, area: Rect, app: &mut App) {
    let category = CATEGORIES[app.settings_selected];
    let selected_item = if app.settings_items_focused() {
        Some(app.settings_item)
    } else {
        None
    };
    let lines = build_detail(category, app, selected_item);

    let inner_width = area.width.saturating_sub(4).max(1) as usize;
    let inner_height = area.height.saturating_sub(3) as usize;
    let total: usize = lines
        .iter()
        .map(|l| l.width().max(1).div_ceil(inner_width))
        .sum();
    let max_scroll = u16::try_from(total.saturating_sub(inner_height)).unwrap_or(u16::MAX);
    app.settings_scroll = app.settings_scroll.min(max_scroll);
    let scroll = app.settings_scroll;

    let mut block = Block::default()
        .title(format!(" {} ", category.label()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding::new(1, 1, 1, 0));
    if total > inner_height {
        let up = if scroll > 0 { "\u{25B2}" } else { " " };
        let down = if scroll as usize + inner_height < total {
            "\u{25BC}"
        } else {
            " "
        };
        block = block.title_bottom(Line::from(format!(" {up} {down} ")).right_aligned());
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, area);
}

fn build_detail(
    category: SettingsCategory,
    app: &App,
    selected_item: Option<usize>,
) -> Vec<Line<'static>> {
    match category {
        SettingsCategory::Network => network_lines(&app.vm.network, selected_item),
        SettingsCategory::Bluetooth => {
            bluetooth_lines(&app.vm.network, app.bt_disconnect_pending(), selected_item)
        }
        SettingsCategory::DeviceInfo => device_info_lines(
            &app.vm.system,
            &app.vm.build,
            app.serial_revealed(),
            selected_item,
        ),
        _ => field_lines(category, app, selected_item),
    }
}

fn field_lines(
    category: SettingsCategory,
    app: &App,
    selected_item: Option<usize>,
) -> Vec<Line<'static>> {
    let system = &app.vm.system;
    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, field) in category_fields(category, bt_connected(&app.vm))
        .iter()
        .enumerate()
    {
        if i > 0 && matches!(field, Field::Timezone) {
            lines.push(Line::raw(""));
        }
        let selected = selected_item == Some(i);
        let row = match field {
            Field::WifiEnabled => toggle_row(field.label(), app.vm.network.wifi.enabled, selected),
            Field::BtDisconnect | Field::UpdateCheck => {
                action_row(field.label(), "press \u{23CE}", Color::Yellow, selected)
            }
            Field::SerialReveal => serial_row(&system.serial, app.serial_revealed(), selected),
            Field::MidiControl => toggle_row(field.label(), system.midi_control, selected),
            Field::AnalyticsUsage => {
                toggle_row(field.label(), system.analytics.share_anom, selected)
            }
            Field::AnalyticsResource => {
                toggle_row(field.label(), system.analytics.share_resource, selected)
            }
            Field::Clock24h => toggle_row(field.label(), system.clock.format_24h, selected),
            Field::ClockDst => toggle_row(field.label(), system.clock.daylight_savings, selected),
            Field::ClockOnHome => toggle_row(field.label(), system.clock.on_home, selected),
            Field::Timezone => {
                let label = timezone_label(u32::try_from(app.resolved_tz_index()).unwrap_or(0));
                select_row(field.label(), &label, selected)
            }
            Field::Language => {
                let label = language_label(language_code(app.resolved_lang_index()));
                select_row(field.label(), label, selected)
            }
            Field::Apply => apply_row(app.settings_category_dirty(), selected),
        };
        lines.push(row);
    }
    if matches!(category, SettingsCategory::Language) {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "  Applying takes a moment \u{2014} the device",
            Style::default().fg(Color::DarkGray),
        ));
        lines.push(Line::styled(
            "  may briefly become unresponsive.",
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines
}

fn toggle_row(label: &str, value: bool, selected: bool) -> Line<'static> {
    let cursor = if selected { "\u{25B8} " } else { "  " };
    let state = if value { "[ On  ]" } else { "[ Off ]" };
    let state_color = if value { Color::Green } else { Color::DarkGray };
    let label_color = if selected { Color::White } else { Color::Gray };
    Line::from(vec![
        Span::styled(cursor, Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{label:<FIELD_LABEL_WIDTH$}"),
            Style::default().fg(label_color),
        ),
        Span::styled(state.to_string(), Style::default().fg(state_color).bold()),
    ])
}

fn select_row(label: &str, value: &str, selected: bool) -> Line<'static> {
    let cursor = if selected { "\u{25B8} " } else { "  " };
    let label_color = if selected { Color::White } else { Color::Gray };
    let mut spans = vec![
        Span::styled(cursor, Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{label:<FIELD_LABEL_WIDTH$}"),
            Style::default().fg(label_color),
        ),
    ];
    if selected {
        spans.push(Span::styled(
            "\u{25C0} ",
            Style::default().fg(Color::Yellow),
        ));
        spans.push(Span::styled(
            value.to_string(),
            Style::default().fg(Color::White).bold(),
        ));
        spans.push(Span::styled(
            " \u{25B6}",
            Style::default().fg(Color::Yellow),
        ));
    } else {
        spans.push(Span::styled(
            value.to_string(),
            Style::default().fg(Color::Gray),
        ));
    }
    Line::from(spans)
}

fn action_row(label: &str, hint: &str, hint_color: Color, selected: bool) -> Line<'static> {
    let cursor = if selected { "\u{25B8} " } else { "  " };
    let label_color = if selected { Color::White } else { Color::Gray };
    Line::from(vec![
        Span::styled(cursor, Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{label:<FIELD_LABEL_WIDTH$}"),
            Style::default().fg(label_color),
        ),
        Span::styled(hint.to_string(), Style::default().fg(hint_color)),
    ])
}

fn apply_row(dirty: bool, selected: bool) -> Line<'static> {
    let (hint, hint_color) = if dirty {
        ("press \u{23CE} to apply", Color::Yellow)
    } else {
        ("no changes", Color::DarkGray)
    };
    action_row("Apply", hint, hint_color, selected)
}

fn network_lines(
    network: &rcp2_core::NetworkInfo,
    selected_item: Option<usize>,
) -> Vec<Line<'static>> {
    let wired = &network.wired;
    let wifi = &network.wifi;
    vec![
        section("Wired"),
        info_row("Connected", yes_no(wired.connected)),
        info_row("IP", or_dash(&wired.ip_address)),
        info_row("Gateway", or_dash(&wired.gateway)),
        info_row("Subnet", or_dash(&wired.subnet_mask)),
        info_row("DNS 1", or_dash(&wired.primary_dns)),
        info_row("DNS 2", or_dash(&wired.secondary_dns)),
        Line::raw(""),
        section("WiFi"),
        toggle_row("Enabled", wifi.enabled, selected_item == Some(0)),
        info_row("SSID", or_dash(&wifi.ssid)),
        info_row("IP", or_dash(&wifi.ip)),
        info_row("DHCP", on_off(wifi.dhcp)),
        info_row("Static IP", yes_no(wifi.static_ip_set)),
    ]
}

fn bluetooth_lines(
    network: &rcp2_core::NetworkInfo,
    disconnecting: bool,
    selected_item: Option<usize>,
) -> Vec<Line<'static>> {
    let bt = &network.bluetooth;
    if bt.connected.is_empty() {
        return vec![
            info_row("Status", "Not connected"),
            info_row("Visible", yes_no(bt.visible)),
        ];
    }
    let (address, name) = bt
        .connected
        .split_once(" - ")
        .map_or((bt.connected.as_str(), ""), |(a, n)| (a, n));
    let status = if disconnecting {
        "Disconnecting\u{2026}"
    } else {
        "Connected"
    };
    let mut lines = vec![info_row("Status", status)];
    if !name.is_empty() {
        lines.push(info_row("Device", name));
    }
    lines.push(info_row("Address", address));
    lines.push(info_row("Visible", yes_no(bt.visible)));
    lines.push(Line::raw(""));
    if disconnecting {
        lines.push(action_row(
            "Disconnect",
            "disconnecting\u{2026}",
            Color::DarkGray,
            selected_item == Some(0),
        ));
    } else {
        lines.push(action_row(
            "Disconnect",
            "press \u{23CE}",
            Color::Yellow,
            selected_item == Some(0),
        ));
    }
    lines
}

fn device_info_lines(
    system: &SystemInfo,
    build: &rcp2_core::BuildInfo,
    serial_revealed: bool,
    selected_item: Option<usize>,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        info_row("Name", or_dash(&system.name)),
        serial_row(&system.serial, serial_revealed, selected_item == Some(0)),
        info_row("Firmware", or_dash(&system.firmware)),
        Line::raw(""),
        section("Build"),
        info_row("Mixer", or_dash(&build.mixer_version)),
        info_row("GUI", or_dash(&build.gui_version)),
        info_row("CallMe", or_dash(&build.callme_version)),
        Line::raw(""),
        section("Update"),
        info_row("Status", &update_status(&system.update)),
    ];
    let (hint, color) = if system.update.checking {
        ("checking\u{2026}", Color::DarkGray)
    } else {
        ("\u{23CE} check for updates", Color::Yellow)
    };
    lines.push(action_row("Check", hint, color, selected_item == Some(1)));
    lines
}

fn update_status(update: &rcp2_core::UpdateInfo) -> String {
    if update.checking {
        "Checking\u{2026}".to_string()
    } else if update.no_internet {
        "No internet".to_string()
    } else if update.available {
        if update.version.is_empty() {
            "Update available".to_string()
        } else {
            format!("Available: {}", update.version)
        }
    } else {
        "Up to date".to_string()
    }
}

fn serial_row(serial: &str, revealed: bool, selected: bool) -> Line<'static> {
    let cursor = if selected { "\u{25B8} " } else { "  " };
    let label_color = if selected {
        Color::White
    } else {
        Color::DarkGray
    };
    let mut spans = vec![
        Span::styled(cursor, Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{:<FIELD_LABEL_WIDTH$}", "Serial"),
            Style::default().fg(label_color),
        ),
    ];
    if serial.is_empty() {
        spans.push(Span::styled("\u{2014}", Style::default().fg(Color::White)));
    } else if revealed {
        spans.push(Span::styled(
            serial.to_string(),
            Style::default().fg(Color::White),
        ));
        if selected {
            spans.push(Span::styled(
                "  \u{23CE} hide",
                Style::default().fg(Color::DarkGray),
            ));
        }
    } else {
        let color = if selected {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        spans.push(Span::styled("toggle to show", Style::default().fg(color)));
    }
    Line::from(spans)
}

fn info_row(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {label:<FIELD_LABEL_WIDTH$}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

fn section(title: &str) -> Line<'static> {
    Line::styled(
        format!("  {title}"),
        Style::default().fg(Color::Cyan).bold(),
    )
}

fn on_off(value: bool) -> &'static str {
    if value { "On" } else { "Off" }
}

fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}

fn or_dash(value: &str) -> &str {
    if value.is_empty() { "\u{2014}" } else { value }
}
