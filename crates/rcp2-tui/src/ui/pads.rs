use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};

use super::util::{detail_kv, hotkey_line, truncate};

pub(super) fn render_pad_list(frame: &mut Frame, area: Rect, app: &App) {
    let bank = app.current_bank();
    let bank_count = app.vm.bank_count();

    let bank_tabs: String = (0..bank_count)
        .map(|i| {
            if i == bank.bank {
                format!("[{}]", i + 1)
            } else {
                format!(" {} ", i + 1)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let inner_width = area.width.saturating_sub(4) as usize;
    let cell_width = inner_width / 2;
    let name_max = cell_width.saturating_sub(5);

    let sep_line = Line::from(vec![
        Span::raw(" ".repeat(cell_width)),
        Span::styled("\u{2502}", Style::default().fg(Color::DarkGray)),
    ]);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(sep_line.clone());

    for row in 0..rcp2_core::PAD_ROWS {
        let mut spans = Vec::new();
        for col in 0..rcp2_core::PAD_COLS {
            let display_idx = row * rcp2_core::PAD_COLS + col;
            let selected = display_idx == app.selected_pad;

            if col > 0 {
                spans.push(Span::styled(
                    "\u{2502}",
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let cursor = if selected { "\u{25B8}" } else { " " };

            if let Some(pad) = &bank.pads[display_idx] {
                let (r, g, b) = pad.color.to_rgb();
                let indicator = if pad.active { "\u{25B6}" } else { " " };
                let display_name = if pad.name.is_empty() {
                    std::path::Path::new(&pad.file_path)
                        .file_stem()
                        .map_or_else(|| "(unnamed)".into(), |n| n.to_string_lossy().into_owned())
                } else {
                    pad.name.clone()
                };
                let name_str = truncate(&display_name, name_max);
                let pad_len = name_max.saturating_sub(name_str.chars().count());

                spans.push(Span::raw(cursor));
                spans.push(Span::styled(
                    indicator,
                    Style::default().fg(if pad.active {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ));
                spans.push(Span::styled(
                    "\u{2588}\u{2588} ",
                    Style::default().fg(Color::Rgb(r, g, b)),
                ));
                spans.push(Span::styled(
                    name_str,
                    Style::default().fg(if selected { Color::White } else { Color::Gray }),
                ));
                spans.push(Span::raw(" ".repeat(pad_len)));
            } else {
                let label = "(empty)";
                let pad_len = name_max.saturating_sub(label.len());
                spans.push(Span::raw(cursor));
                spans.push(Span::raw("    "));
                spans.push(Span::styled(label, Style::default().fg(Color::DarkGray)));
                spans.push(Span::raw(" ".repeat(pad_len)));
            }
        }
        lines.push(Line::from(spans));
        if row < rcp2_core::PAD_ROWS - 1 {
            lines.push(sep_line.clone());
        }
    }

    let list = Paragraph::new(lines).block(
        Block::default()
            .title(format!(" Bank {bank_tabs} "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .padding(Padding::new(1, 1, 0, 0)),
    );

    frame.render_widget(list, area);
}

pub(super) fn render_pad_detail(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(ref form) = app.detail_form {
        super::form::render_detail_form(frame, area, app, form);
        return;
    }

    let lines = if let Some(pad) = app.selected_pad_info() {
        let (r, g, b) = pad.color.to_rgb();
        let mode = crate::detail_form::play_mode_label(pad.play_mode);
        let mut l = vec![Line::from(vec![
            Span::styled(
                "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588} ",
                Style::default().fg(Color::Rgb(r, g, b)),
            ),
            Span::styled(pad.name.clone(), Style::default().fg(Color::White).bold()),
        ])];
        if pad.active {
            l.push(Line::from(vec![
                Span::styled(
                    "\u{25B6} PLAYING ",
                    Style::default().fg(Color::Green).bold(),
                ),
                Span::styled(
                    format!("{:.0}%", pad.progress * 100.0),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }
        l.push(Line::raw(""));
        l.push(detail_kv("Type", pad.pad_type.label()));
        l.push(detail_kv("Mode", &mode));
        l.push(detail_kv("Gain", &format!("{:.1} dB", pad.gain)));
        l.push(detail_kv("Loop", if pad.looping { "Yes" } else { "No" }));
        l.push(detail_kv("Replay", if pad.replay { "Yes" } else { "No" }));
        if !pad.file_path.is_empty() {
            l.push(detail_kv(
                "Start",
                &format!("{:.0}%", pad.env_start * 100.0),
            ));
            l.push(detail_kv("End", &format!("{:.0}%", pad.env_stop * 100.0)));
            l.push(Line::raw(""));
            l.push(Line::from(Span::styled(
                "File",
                Style::default().fg(Color::DarkGray),
            )));
            l.push(Line::from(Span::styled(
                pad.file_path.clone(),
                Style::default().fg(Color::White),
            )));
        }
        if app.allow_send {
            l.push(Line::raw(""));
            l.push(hotkey_line("p", "play/stop"));
            l.push(hotkey_line("\u{23CE}", "edit pad"));
        }
        l
    } else {
        let mut l = vec![
            Line::styled("(empty)", Style::default().fg(Color::DarkGray)),
            Line::raw(""),
        ];
        if app.allow_send {
            l.push(hotkey_line("\u{23CE}", "new pad"));
        }
        l
    };

    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding::new(1, 1, 1, 0));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
