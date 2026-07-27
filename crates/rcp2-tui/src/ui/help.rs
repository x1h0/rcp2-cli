use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Padding, Paragraph};

use super::util::render_scrollbar;

use super::util::hotkey_line;

pub(super) fn render_help(frame: &mut Frame, area: Rect, app: &mut App) {
    let lines = vec![
        Line::styled("  Pads View", Style::default().fg(Color::Cyan).bold()),
        hotkey_line("\u{2190}\u{2192}", "switch bank"),
        hotkey_line("\u{2191}\u{2193}", "select pad"),
        hotkey_line(
            &format!("1-{}", app.profile.pads_per_bank),
            "select pad directly",
        ),
        hotkey_line("p", "play/stop"),
        hotkey_line("\u{23CE}", "edit pad"),
        hotkey_line("PgUp/PgDn", "scroll detail"),
        Line::raw(""),
        Line::styled("  Edit View", Style::default().fg(Color::Cyan).bold()),
        hotkey_line("\u{2191}\u{2193}", "navigate fields"),
        hotkey_line("\u{23CE}", "edit/toggle/action"),
        hotkey_line("\u{2190}\u{2192}", "cycle options"),
        hotkey_line("Esc", "back to pads"),
        Line::raw(""),
        Line::styled("  Monitor", Style::default().fg(Color::Cyan).bold()),
        hotkey_line("\u{2191}\u{2193}", "scroll"),
        hotkey_line("s", "save log to file"),
        Line::raw(""),
        Line::styled("  Transfer", Style::default().fg(Color::Cyan).bold()),
        hotkey_line("1/2", "internal/sd card"),
        hotkey_line("\u{2191}\u{2193}", "browse files"),
        hotkey_line("\u{23CE}", "open dir / download file"),
        hotkey_line("d", "download file or folder"),
        Line::raw(""),
        Line::styled("  Settings", Style::default().fg(Color::Cyan).bold()),
        hotkey_line("\u{2191}\u{2193}", "category / field"),
        hotkey_line("\u{2190}\u{2192}", "change value"),
        hotkey_line("\u{23CE}", "open / toggle"),
        hotkey_line("PgUp/PgDn", "scroll details"),
        hotkey_line("Esc", "back"),
        Line::raw(""),
        Line::styled("  Global", Style::default().fg(Color::Cyan).bold()),
        hotkey_line("m", "toggle monitor"),
        hotkey_line("s", "device settings"),
        hotkey_line("r", "record/pause"),
        hotkey_line("R", "stop recording"),
        hotkey_line("t", "transfer mode"),
        hotkey_line("?", "this help"),
        hotkey_line("q", "quit"),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::new(1, 1, 1, 0));

    let line_count = lines.len();
    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.help_scroll, 0));
    frame.render_widget(paragraph, area);

    let inner_height = usize::from(area.height.saturating_sub(3));
    app.help_max_scroll =
        u16::try_from(line_count.saturating_sub(inner_height)).unwrap_or(u16::MAX);
    render_scrollbar(
        frame,
        area,
        line_count,
        inner_height,
        usize::from(app.help_scroll),
    );
}
