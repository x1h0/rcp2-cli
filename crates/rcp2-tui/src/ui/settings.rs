use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use rcp2_core::SystemInfo;
use rcp2_core::settings::{
    CATEGORIES, Field, SettingsCategory, bt_connected, category_fields, language_code,
    language_label, timezone_label,
};

const FIELD_LABEL_WIDTH: usize = 13;

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
