use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, List, ListItem, Padding, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

pub(super) fn render_monitor(frame: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let total = app.event_log.len();
    let skip = total.saturating_sub(inner_height.saturating_add(app.log_scroll));

    let items: Vec<ListItem> = app
        .event_log
        .iter()
        .skip(skip)
        .take(inner_height)
        .map(|entry| {
            let style = if entry.starts_with("[update]") {
                Style::default().fg(Color::Gray)
            } else if entry.starts_with("[state]") {
                Style::default().fg(Color::Green)
            } else if entry.starts_with("[error]") {
                Style::default().fg(Color::Red)
            } else if entry.starts_with("[unknown]") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            ListItem::new(Span::styled(entry.clone(), style))
        })
        .collect();

    let title = format!(" Monitor ({}) ", app.log_total);
    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .padding(Padding::new(1, 1, 0, 0)),
    );

    frame.render_widget(list, area);

    if total > inner_height {
        let max_scroll = total.saturating_sub(inner_height);
        let position = max_scroll.saturating_sub(app.log_scroll);
        let mut state = ScrollbarState::new(max_scroll).position(position);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, area, &mut state);
    }
}
