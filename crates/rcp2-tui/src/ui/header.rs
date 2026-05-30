use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use rcp2_core::RecordingStatus;

use super::util::format_seconds;

pub(super) fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let vm = &app.vm;
    let sep = Style::default().fg(Color::DarkGray);

    let rec_style = match vm.recorder.state {
        RecordingStatus::Recording => Style::default().fg(Color::Red).bold(),
        RecordingStatus::Paused => Style::default().fg(Color::Yellow).bold(),
        RecordingStatus::Stopped => Style::default().fg(Color::DarkGray),
    };

    let rec_time = format_seconds(app.recording_seconds());

    let has_storage = vm.has_storage();

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
            if has_storage {
                let sd = vm.storage.iter().find(|s| s.is_available());
                match sd {
                    Some(s) if s.capacity_bytes > 0 => {
                        let used = s.capacity_bytes.saturating_sub(s.free_bytes);
                        format!(
                            "SD \u{2713} {}/{}",
                            rcp2_core::ops::format_size(used),
                            rcp2_core::ops::format_size(s.capacity_bytes),
                        )
                    }
                    _ => "SD \u{2713}".into(),
                }
            } else {
                "SD \u{2717}".into()
            },
            Style::default().fg(if has_storage {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
    ];

    if vm.network.wifi_enabled && !vm.network.wifi_ssid.is_empty() {
        spans.push(Span::styled(" \u{2502} ", sep));
        spans.push(Span::styled(
            format!("WiFi: {}", vm.network.wifi_ssid),
            Style::default().fg(Color::Cyan),
        ));
    } else if vm.network.wired {
        spans.push(Span::styled(" \u{2502} ", sep));
        spans.push(Span::styled("ETH", Style::default().fg(Color::Green)));
    }

    if !vm.network.bt_connected.is_empty() {
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
