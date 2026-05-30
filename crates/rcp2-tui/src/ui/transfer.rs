use crate::app::App;
use crate::transfer::{PadDownloadState, PadUploadState, TransferStatus, format_size};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap};

use super::util::render_scrollbar;

pub(super) fn render_transfer(frame: &mut Frame, area: Rect, app: &App) {
    if app.transfer.storage_choice.is_none() {
        render_storage_choice(frame, area);
        return;
    }

    match app.transfer.status {
        TransferStatus::Inactive => render_transfer_status(
            frame,
            area,
            Color::DarkGray,
            &[
                "  Transfer mode is inactive.",
                "  Press 't' from the pads view to activate.",
            ],
        ),
        TransferStatus::Activating => render_transfer_status(
            frame,
            area,
            Color::Yellow,
            &[
                "  Activating transfer mode...",
                "  Waiting for mass storage device to appear.",
            ],
        ),
        TransferStatus::Error => {
            let msg = format!("  Error: {}", app.transfer.message);
            render_transfer_status(frame, area, Color::Red, &[&msg]);
        }
        TransferStatus::Active => {
            if app.transfer.save_prompt.is_some() {
                render_transfer_save_prompt(frame, area, app);
            } else {
                render_transfer_file_list(frame, area, app);
            }
        }
    }
}

fn render_storage_choice(frame: &mut Frame, area: Rect) {
    let text = Paragraph::new(vec![
        Line::raw(""),
        Line::raw(""),
        Line::styled(
            "  Select storage:",
            Style::default().fg(Color::White).bold(),
        ),
        Line::raw(""),
        Line::from(vec![
            Span::styled("    1 ", Style::default().fg(Color::Yellow)),
            Span::styled("Internal (eMMC)", Style::default().fg(Color::White)),
            Span::styled(
                "  \u{2014} pads, system data",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    2 ", Style::default().fg(Color::Yellow)),
            Span::styled("SD Card", Style::default().fg(Color::White)),
            Span::styled(
                "  \u{2014} recordings, scene exports",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::raw(""),
        Line::styled(
            "  \u{26A0} Transfer mode will be activated.",
            Style::default().fg(Color::Yellow),
        ),
        Line::styled(
            "  \u{26A0} Device audio & pads will be unavailable.",
            Style::default().fg(Color::Yellow),
        ),
        Line::raw(""),
        Line::styled("  Esc to cancel", Style::default().fg(Color::DarkGray)),
    ])
    .block(
        Block::default()
            .title(" Transfer \u{2014} Choose Storage ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(text, area);
}

fn render_transfer_status(frame: &mut Frame, area: Rect, color: Color, messages: &[&str]) {
    let mut lines = vec![Line::raw("")];
    for msg in messages {
        lines.push(Line::styled(*msg, Style::default().fg(color)));
    }
    let text = Paragraph::new(lines).block(
        Block::default()
            .title(" Transfer ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color)),
    );
    frame.render_widget(text, area);
}

pub(super) fn render_transfer_file_list(frame: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(3) as usize; // borders + padding
    let total = app.transfer.files.len();
    let selected = app.transfer.selected;

    let scroll = if selected >= inner_height {
        selected - inner_height + 1
    } else {
        0
    };

    let title = format!(" Transfer: {} ({} files) ", app.transfer.current_dir, total);

    let items: Vec<ListItem> = app
        .transfer
        .files
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner_height)
        .map(|(i, entry)| {
            let is_sel = i == selected;
            let icon = if entry.is_dir {
                "\u{1F4C1}"
            } else {
                "\u{1F4C4}"
            };
            let size = if entry.is_dir {
                String::new()
            } else {
                format!("  {}", format_size(entry.size))
            };

            Line::from(vec![
                Span::raw(if is_sel { "\u{25B8} " } else { "  " }),
                Span::raw(format!("{icon} ")),
                Span::styled(
                    entry.name.clone(),
                    Style::default().fg(if is_sel {
                        Color::White
                    } else if entry.is_dir {
                        Color::Cyan
                    } else {
                        Color::Gray
                    }),
                ),
                Span::styled(size, Style::default().fg(Color::DarkGray)),
            ])
        })
        .map(ListItem::new)
        .collect();

    let mut block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .padding(Padding::new(1, 1, 0, 0));

    if !app.transfer.message.is_empty() {
        block = block.title_bottom(format!(" {} ", app.transfer.message));
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, area);

    render_scrollbar(frame, area, total, inner_height, scroll);
}

pub(super) fn render_transfer_save_prompt(frame: &mut Frame, area: Rect, app: &App) {
    let prompt = app.transfer.save_prompt.as_ref().map(|p| &p.input);
    let input = prompt.map_or("", std::string::String::as_str);

    let source_name = app
        .transfer
        .save_prompt
        .as_ref()
        .and_then(|p| p.source.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(5)])
        .split(area);

    render_transfer_file_list(frame, layout[0], app);

    let prompt_block = Block::default()
        .title(format!(" Save: {source_name} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .padding(Padding::new(1, 1, 0, 0));

    let text = vec![
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
            Span::styled(input, Style::default().fg(Color::White)),
            Span::styled("\u{258C}", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(Span::styled(
            "Enter: save  Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text).block(prompt_block);
    frame.render_widget(paragraph, layout[1]);
}

pub(super) fn render_pad_download_overlay(
    frame: &mut Frame,
    area: Rect,
    dl: &crate::transfer::PadDownload,
) {
    let (title, border_color, lines) = match dl.state {
        PadDownloadState::Prompting => {
            let filename = std::path::Path::new(&dl.device_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            (
                format!(" Download: {filename} "),
                Color::Yellow,
                vec![
                    Line::raw(""),
                    Line::from(vec![
                        Span::styled("  Save to: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(&dl.prompt.input, Style::default().fg(Color::White)),
                        Span::styled("\u{258C}", Style::default().fg(Color::Yellow)),
                    ]),
                    Line::raw(""),
                    Line::styled(
                        "  Enter: download  Esc: cancel",
                        Style::default().fg(Color::DarkGray),
                    ),
                ],
            )
        }
        PadDownloadState::Activating | PadDownloadState::WaitingForMount => (
            " Download ".to_string(),
            Color::Yellow,
            vec![
                Line::raw(""),
                Line::styled(
                    "  Activating transfer mode...",
                    Style::default().fg(Color::Yellow),
                ),
                Line::styled(
                    "  Waiting for device storage to mount.",
                    Style::default().fg(Color::DarkGray),
                ),
            ],
        ),
        PadDownloadState::Copying => (
            " Download ".to_string(),
            Color::Green,
            vec![
                Line::raw(""),
                Line::styled("  Copying file...", Style::default().fg(Color::Green)),
            ],
        ),
        PadDownloadState::Deactivating => (
            " Download ".to_string(),
            Color::Cyan,
            vec![
                Line::raw(""),
                Line::styled(
                    "  Deactivating transfer mode...",
                    Style::default().fg(Color::Cyan),
                ),
            ],
        ),
        PadDownloadState::Done => (
            " Download ".to_string(),
            Color::Green,
            vec![
                Line::raw(""),
                Line::styled(
                    format!("  {}", dl.message),
                    Style::default().fg(Color::Green),
                ),
            ],
        ),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

pub(super) fn render_pad_upload_overlay(
    frame: &mut Frame,
    area: Rect,
    ul: &crate::transfer::PadUpload,
) {
    let (title, border_color, lines) = match ul.state {
        PadUploadState::Prompting => (
            " Upload Sound ".to_string(),
            Color::Yellow,
            vec![
                Line::raw(""),
                Line::from(vec![
                    Span::styled("  File: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&ul.prompt.input, Style::default().fg(Color::White)),
                    Span::styled("\u{258C}", Style::default().fg(Color::Yellow)),
                ]),
                Line::raw(""),
                Line::styled(
                    "  Enter: upload  Esc: cancel",
                    Style::default().fg(Color::DarkGray),
                ),
            ],
        ),
        PadUploadState::Activating | PadUploadState::WaitingForMount => (
            " Upload ".to_string(),
            Color::Yellow,
            vec![
                Line::raw(""),
                Line::styled(
                    "  Activating transfer mode...",
                    Style::default().fg(Color::Yellow),
                ),
            ],
        ),
        PadUploadState::Copying => (
            " Upload ".to_string(),
            Color::Green,
            vec![
                Line::raw(""),
                Line::styled("  Uploading file...", Style::default().fg(Color::Green)),
            ],
        ),
        _ => (
            " Upload ".to_string(),
            Color::Cyan,
            vec![
                Line::raw(""),
                Line::styled(
                    format!("  {}", ul.message),
                    Style::default().fg(Color::Cyan),
                ),
            ],
        ),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
