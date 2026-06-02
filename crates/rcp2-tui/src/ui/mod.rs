mod form;
mod header;
mod help;
mod monitor;
mod pads;
mod settings;
mod settings_labels;
mod strip;
mod transfer;
mod util;

pub(crate) use settings::{
    DraftField, FieldRole, SETTINGS_CATEGORY_COUNT, SettingsNode, SettingsStep,
    category_draft_fields, cycle_language, cycle_timezone, language_code, language_index,
    settings_category_is_language, settings_field_role, settings_item_count, settings_live_toggle,
};

use crate::app::{App, MainView};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    header::render_header(frame, outer[0], app);
    strip::render_device_strip(frame, outer[1], app);
    render_main(frame, outer[2], app);
    render_status(frame, outer[3], app);
}

pub fn render_connecting(frame: &mut Frame, area: Rect) {
    let text = Paragraph::new("Connecting to R\u{00D8}DECaster...")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    let [_, center, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);

    frame.render_widget(text, center);
}

pub fn render_disclaimer(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Warning ")
        .title_style(Style::default().fg(Color::Red).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .padding(Padding::new(2, 2, 1, 1));

    let warn = Style::default().fg(Color::Yellow);
    let dim = Style::default().fg(Color::DarkGray);
    let bold = Style::default().fg(Color::White).bold();

    let mut lines = vec![
        Line::styled(
            "EXPERIMENTAL SOFTWARE",
            Style::default().fg(Color::Red).bold(),
        ),
        Line::raw(""),
        Line::styled(
            "This tool communicates directly with your R\u{00D8}DECaster",
            warn,
        ),
        Line::styled("via USB HID using a reverse-engineered protocol.", warn),
        Line::raw(""),
        Line::styled("Known issue:", bold),
        Line::styled("  After closing this app, device buttons may freeze.", warn),
        Line::styled("  Replug the USB cable to recover.", warn),
        Line::raw(""),
        Line::styled("This tool can write to the device. Additionally:", bold),
        Line::raw(""),
        Line::styled("  - Sending data MAY freeze or brick your device", warn),
        Line::styled("  - Your configuration or sounds could be corrupted", warn),
    ];

    lines.push(Line::raw(""));
    lines.push(Line::styled("  No warranty. Use at your own risk.", warn));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("Press ", dim),
        Span::styled("y", Style::default().fg(Color::Green).bold()),
        Span::styled(" to accept and continue, ", dim),
        Span::styled("q", Style::default().fg(Color::Red).bold()),
        Span::styled(" to quit", dim),
    ]));
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "Tip: --i-know-what-i-do or RCP2_ACCEPT_RISK=1 to skip",
        dim,
    ));

    let height = 24;

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Center);

    let [_, center, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, inner, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(66),
        Constraint::Fill(1),
    ])
    .areas(center);

    frame.render_widget(paragraph, inner);
}

fn render_main(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.modal == crate::app::ModalState::Help {
        help::render_help(frame, area, app);
        return;
    }

    if let Some(ref dialog) = app.confirm_dialog {
        let block = Block::default()
            .title(format!(" {} ", dialog.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .padding(Padding::new(2, 2, 1, 0));

        let mut lines: Vec<Line> = vec![Line::raw("")];
        for line in dialog.message.lines() {
            let style = if line.contains('\u{26A0}') {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::styled(line.to_string(), style));
        }
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  y/Enter ", Style::default().fg(Color::Red).bold()),
            Span::raw("confirm    "),
            Span::styled("n/Esc ", Style::default().fg(Color::Green).bold()),
            Span::raw("cancel"),
        ]));

        let text = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(text, area);
        return;
    }

    match app.main_view {
        MainView::Pads => {
            if app.modal == crate::app::ModalState::WaitingForPadPress {
                let text = Paragraph::new(vec![
                    Line::raw(""),
                    Line::raw(""),
                    Line::styled(
                        "  Press the pad button on your RodeCaster",
                        Style::default().fg(Color::Yellow).bold(),
                    ),
                    Line::styled(
                        "  that you want to configure.",
                        Style::default().fg(Color::Yellow).bold(),
                    ),
                    Line::raw(""),
                    Line::styled("  Esc to cancel", Style::default().fg(Color::DarkGray)),
                ])
                .block(
                    Block::default()
                        .title(" New Pad ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                );
                frame.render_widget(text, area);
            } else if let Some(ref mv) = app.pad_move {
                transfer::render_pad_move_overlay(frame, area, mv);
            } else if let Some(ref ul) = app.pad_upload {
                transfer::render_pad_upload_overlay(frame, area, ul);
            } else if let Some(ref dl) = app.pad_download {
                transfer::render_pad_download_overlay(frame, area, dl);
            } else {
                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(30), Constraint::Length(40)])
                    .split(area);
                pads::render_pad_list(frame, layout[0], app);
                pads::render_pad_detail(frame, layout[1], app);
            }
        }
        MainView::Monitor => {
            monitor::render_monitor(frame, area, app);
        }
        MainView::Transfer => {
            transfer::render_transfer(frame, area, app);
        }
        MainView::Settings => {
            settings::render_settings(frame, area, app);
        }
    }
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::raw(" ")];

    let view_label = match app.main_view {
        MainView::Pads if app.move_selection.is_some() => "Move",
        MainView::Pads if app.detail_form.is_some() => "Edit",
        MainView::Pads => "Pads",
        MainView::Monitor => "Monitor",
        MainView::Transfer => "Transfer",
        MainView::Settings => "Settings",
    };

    spans.push(Span::styled(
        format!("[{view_label}] "),
        Style::default().fg(Color::Cyan),
    ));

    if app.move_selection.is_some() {
        spans.extend([
            Span::styled(
                "\u{2192}\u{2191}\u{2193} ",
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("pick empty slot  "),
            Span::styled("\u{23CE} ", Style::default().fg(Color::Yellow)),
            Span::raw("confirm  "),
            Span::styled("Esc ", Style::default().fg(Color::Yellow)),
            Span::raw("cancel  "),
            Span::styled(
                format!("\u{2502} {} ", app.status),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        let status = Paragraph::new(Line::from(spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(status, area);
        return;
    }

    match app.main_view {
        MainView::Pads => {
            spans.extend([
                Span::styled("m ", Style::default().fg(Color::Yellow)),
                Span::raw("monitor  "),
                Span::styled("s ", Style::default().fg(Color::Yellow)),
                Span::raw("settings  "),
            ]);
            if !app.dry_run {
                spans.extend([
                    Span::styled("t ", Style::default().fg(Color::Yellow)),
                    Span::raw("transfer  "),
                ]);
            }
        }
        MainView::Settings => {
            spans.extend([
                Span::styled("\u{2190}\u{2192} ", Style::default().fg(Color::Yellow)),
                Span::raw("change  "),
                Span::styled("\u{23CE} ", Style::default().fg(Color::Yellow)),
                Span::raw("toggle  "),
                Span::styled("Esc ", Style::default().fg(Color::Yellow)),
                Span::raw("back  "),
            ]);
        }
        MainView::Monitor | MainView::Transfer => {
            spans.extend([
                Span::styled("Esc ", Style::default().fg(Color::Yellow)),
                Span::raw("pads  "),
            ]);
        }
    }

    spans.extend([
        Span::styled("? ", Style::default().fg(Color::Yellow)),
        Span::raw("help  "),
        Span::styled("q ", Style::default().fg(Color::Yellow)),
        Span::raw("quit  "),
        Span::styled(
            format!("\u{2502} {} ", app.status),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let status = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(status, area);
}
