use ratatui::prelude::*;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

pub(super) fn hotkey_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<4} "), Style::default().fg(Color::Yellow)),
        Span::styled(desc.to_string(), Style::default().fg(Color::DarkGray)),
    ])
}

pub(super) fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    total: usize,
    viewport: usize,
    position: usize,
) {
    if total > viewport {
        let max_scroll = total - viewport;
        let mut state = ScrollbarState::new(max_scroll).position(position);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, area, &mut state);
    }
}

pub(super) fn detail_kv(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<10}"), Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{truncated}\u{2026}")
    }
}

pub(super) fn level_bar(value: f64, max: f64) -> String {
    if max <= 0.0 {
        return "\u{2591}\u{2591}\u{2591}\u{2591}".to_string();
    }
    let ratio = (value / max).clamp(0.0, 1.0);
    let eighths = (1..=32).filter(|&n| ratio * 32.0 >= f64::from(n)).count();
    let filled = eighths / 8;
    let partial = eighths % 8;
    let blocks = [
        '\u{2591}', '\u{2592}', '\u{2592}', '\u{2592}', '\u{2593}', '\u{2593}', '\u{2593}',
        '\u{2593}',
    ];

    let mut bar = String::new();
    for i in 0..4 {
        if i < filled {
            bar.push('\u{2588}');
        } else if i == filled && partial > 0 {
            bar.push(blocks[partial]);
        } else {
            bar.push('\u{2591}');
        }
    }
    bar
}

pub(super) fn format_seconds(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::level_bar;

    #[test]
    fn level_bar_covers_the_range() {
        assert_eq!(level_bar(0.0, 1.0), "░░░░");
        assert_eq!(level_bar(0.5, 1.0), "██░░");
        assert_eq!(level_bar(0.55, 1.0), "██▒░");
        assert_eq!(level_bar(0.625, 1.0), "██▓░");
        assert_eq!(level_bar(1.0, 1.0), "████");
        assert_eq!(level_bar(127.0, 127.0), "████");
    }

    #[test]
    fn level_bar_handles_degenerate_input() {
        assert_eq!(level_bar(1.0, 0.0), "░░░░");
        assert_eq!(level_bar(-5.0, 1.0), "░░░░");
        assert_eq!(level_bar(5.0, 1.0), "████");
    }
}
