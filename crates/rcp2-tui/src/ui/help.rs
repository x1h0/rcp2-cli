use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Padding, Paragraph};

use super::util::hotkey_line;

pub(super) fn render_help(frame: &mut Frame, area: Rect, app: &App) {
    let send = app.allow_send;
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines = vec![
        Line::styled("  Pads View", Style::default().fg(Color::Cyan).bold()),
        hotkey_line("\u{2190}\u{2192}", "switch bank"),
        hotkey_line("\u{2191}\u{2193}", "select pad"),
        hotkey_line("1-8", "select pad directly"),
    ];

    if send {
        lines.push(hotkey_line("p", "play/stop"));
        lines.push(hotkey_line("\u{23CE}", "edit pad"));
    }

    lines.push(Line::raw(""));

    if send {
        lines.push(Line::styled(
            "  Edit View",
            Style::default().fg(Color::Cyan).bold(),
        ));
        lines.push(hotkey_line("\u{2191}\u{2193}", "navigate fields"));
        lines.push(hotkey_line("\u{23CE}", "edit/toggle/action"));
        lines.push(hotkey_line("\u{2190}\u{2192}", "cycle options"));
        lines.push(hotkey_line("Esc", "back to pads"));
        lines.push(Line::raw(""));
    }

    lines.push(Line::styled(
        "  Monitor",
        Style::default().fg(Color::Cyan).bold(),
    ));
    lines.push(hotkey_line("\u{2191}\u{2193}", "scroll"));
    lines.push(hotkey_line("s", "save log to file"));
    lines.push(Line::raw(""));

    if send {
        lines.push(Line::styled(
            "  Transfer",
            Style::default().fg(Color::Cyan).bold(),
        ));
        lines.push(hotkey_line("1/2", "internal/sd card"));
        lines.push(hotkey_line("\u{2191}\u{2193}", "browse files"));
        lines.push(hotkey_line("\u{23CE}", "open dir / download file"));
        lines.push(hotkey_line("d", "download file or folder"));
        lines.push(Line::raw(""));
    }

    lines.push(Line::styled(
        "  Global",
        Style::default().fg(Color::Cyan).bold(),
    ));
    lines.push(hotkey_line("m", "toggle monitor"));
    if send {
        lines.push(hotkey_line("r", "record/pause"));
        lines.push(hotkey_line("R", "stop recording"));
        lines.push(hotkey_line("t", "transfer mode"));
    }
    lines.push(hotkey_line("?", "this help"));
    lines.push(hotkey_line("q", "quit"));

    if !send {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "  Start with --allow-send for full access",
            dim,
        ));
    }

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::new(1, 1, 1, 0));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.help_scroll, 0));
    frame.render_widget(paragraph, area);
}
