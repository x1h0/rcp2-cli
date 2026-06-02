use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use rcp2_core::RecordingStatus;

use super::util::format_seconds;
use rcp2_core::StorageInfo;

fn internal_label(info: Option<&StorageInfo>) -> String {
    match info {
        Some(s) if s.capacity_bytes > 0 => {
            let used = s.capacity_bytes.saturating_sub(s.free_bytes);
            format!(
                "Int {}/{}",
                rcp2_core::ops::format_size(used),
                rcp2_core::ops::format_size(s.capacity_bytes),
            )
        }
        _ => "Int".into(),
    }
}

fn storage_label(name: &str, info: Option<&StorageInfo>) -> String {
    match info {
        Some(s) if s.capacity_bytes > 0 => {
            let used = s.capacity_bytes.saturating_sub(s.free_bytes);
            format!(
                "{name} \u{2713} {}/{}",
                rcp2_core::ops::format_size(used),
                rcp2_core::ops::format_size(s.capacity_bytes),
            )
        }
        Some(_) => format!("{name} \u{2713}"),
        None => format!("{name} \u{2717}"),
    }
}

pub(super) fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let vm = &app.vm;
    let sep = Style::default().fg(Color::DarkGray);

    let rec_style = match vm.recorder.state {
        RecordingStatus::Recording => Style::default().fg(Color::Red).bold(),
        RecordingStatus::Paused => Style::default().fg(Color::Yellow).bold(),
        RecordingStatus::Stopped => Style::default().fg(Color::DarkGray),
    };

    let rec_time = format_seconds(app.recording_seconds());

    let title_style = if app.connected {
        Style::default().fg(Color::Red).bold()
    } else {
        Style::default().fg(Color::DarkGray).bold()
    };

    let title = if app.connected {
        format!(" {} ", app.profile.display_name)
    } else {
        format!(" {} [DISCONNECTED] ", app.profile.display_name)
    };
    let mut spans = vec![
        Span::styled(title, title_style),
        Span::styled("\u{2502} ", sep),
        Span::styled(&vm.system.firmware, Style::default().fg(Color::DarkGray)),
        Span::styled(" \u{2502} ", sep),
        Span::styled(&vm.show.name, Style::default().fg(Color::Cyan)),
        Span::styled(" \u{2502} ", sep),
        Span::styled(
            format!("{} {rec_time}", vm.recorder.state.label()),
            rec_style,
        ),
        Span::styled(" \u{2502} ", sep),
        Span::styled(
            internal_label(vm.internal_storage()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(" \u{2502} ", sep),
        Span::styled(
            storage_label("SD", vm.sd_storage()),
            Style::default().fg(if vm.sd_storage().is_some() {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
    ];

    if app.dry_run {
        spans.insert(
            1,
            Span::styled("[DRY-RUN] ", Style::default().fg(Color::Yellow).bold()),
        );
    }

    if vm.network.wifi.enabled && !vm.network.wifi.ssid.is_empty() {
        spans.push(Span::styled(" \u{2502} ", sep));
        spans.push(Span::styled(
            format!("WiFi: {}", vm.network.wifi.ssid),
            Style::default().fg(Color::Cyan),
        ));
    } else if vm.network.wired.connected {
        spans.push(Span::styled(" \u{2502} ", sep));
        spans.push(Span::styled("ETH", Style::default().fg(Color::Green)));
    }

    if !vm.network.bluetooth.connected.is_empty() {
        spans.push(Span::styled(" \u{2502} ", sep));
        spans.push(Span::styled("BT", Style::default().fg(Color::Blue)));
    }

    let header = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}
