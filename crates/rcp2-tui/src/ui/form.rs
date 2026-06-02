use crate::app::App;
use crate::detail_form::{DetailForm, FieldKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Padding, Paragraph};

pub(super) fn render_detail_form(frame: &mut Frame, area: Rect, app: &App, form: &DetailForm) {
    let mut lines: Vec<Line> = vec![];

    let is_new = form.new_pad_idx.is_some();
    let title = if form.is_replace {
        " Replace Sound "
    } else if is_new {
        " New Pad "
    } else {
        " Edit Pad "
    };

    render_form_header(&mut lines, app, form, is_new);

    let mut actions_started = false;
    let mut selected_line = 0usize;
    for (i, field) in form.fields.iter().enumerate() {
        let selected = i == form.selected;
        render_field(
            &mut lines,
            app,
            form,
            field,
            selected,
            is_new,
            &mut actions_started,
        );
        if selected {
            selected_line = lines.len().saturating_sub(1);
        }
    }

    let inner_height = area.height.saturating_sub(3) as usize;
    let total = lines.len();
    let scroll = if inner_height > 0 && selected_line >= inner_height {
        selected_line + 1 - inner_height
    } else {
        0
    };

    let mut block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::new(1, 1, 1, 0));
    if inner_height > 0 && total > inner_height {
        let up = if scroll > 0 { "\u{25B2}" } else { " " };
        let down = if scroll + inner_height < total {
            "\u{25BC}"
        } else {
            " "
        };
        block = block.title_bottom(Line::from(format!(" {up} {down} ")).right_aligned());
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((u16::try_from(scroll).unwrap_or(0), 0));
    frame.render_widget(paragraph, area);
}

fn render_form_header(lines: &mut Vec<Line>, app: &App, form: &DetailForm, is_new: bool) {
    if !form.is_replace && is_new {
        let (r, g, b) = rcp2_core::PadColor::from_index(form.new_pad_color).to_rgb();
        lines.push(Line::from(vec![
            Span::styled(
                "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588} ",
                Style::default().fg(Color::Rgb(r, g, b)),
            ),
            Span::styled("(new pad)", Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::raw(""));
    } else if !form.is_replace
        && let Some(pad) = app.selected_pad_info()
    {
        let (r, g, b) = pad.color.to_rgb();
        lines.push(Line::from(vec![
            Span::styled(
                "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588} ",
                Style::default().fg(Color::Rgb(r, g, b)),
            ),
            Span::styled(pad.name.clone(), Style::default().fg(Color::White).bold()),
        ]));
        if pad.active {
            lines.push(Line::from(vec![
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
        lines.push(Line::raw(""));
    }
}

fn render_field<'a>(
    lines: &mut Vec<Line<'a>>,
    app: &App,
    form: &'a DetailForm,
    field: &'a crate::detail_form::FormField,
    selected: bool,
    is_new: bool,
    actions_started: &mut bool,
) {
    let cursor = if selected { "\u{25B8} " } else { "  " };

    match field.kind {
        FieldKind::Action => {
            if !*actions_started {
                *actions_started = true;
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "  \u{2500}\u{2500}\u{2500} Actions \u{2500}\u{2500}\u{2500}",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            lines.push(Line::from(vec![
                Span::raw(cursor),
                Span::styled(
                    &field.label,
                    Style::default().fg(if selected { Color::White } else { Color::Gray }),
                ),
            ]));
        }
        FieldKind::Text | FieldKind::Number if selected && form.is_editing() => {
            let text = form.editing_text.as_ref().map_or("", |e| e.input.as_str());
            lines.push(Line::from(vec![
                Span::raw(cursor),
                Span::styled(
                    format!("{:<9}", field.label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(text, Style::default().fg(Color::White)),
                Span::styled("\u{258C}", Style::default().fg(Color::Yellow)),
            ]));
        }
        FieldKind::ColorCycle => {
            let (r, g, b) = if is_new {
                rcp2_core::PadColor::from_index(form.new_pad_color).to_rgb()
            } else if let Some(p) = app.selected_pad_info() {
                p.color.to_rgb()
            } else {
                (128, 128, 128)
            };
            let hint = if selected { " \u{25C2} \u{25B8}" } else { "" };
            lines.push(Line::from(vec![
                Span::raw(cursor),
                Span::styled(
                    format!("{:<9}", field.label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    "\u{2588}\u{2588} ",
                    Style::default().fg(Color::Rgb(r, g, b)),
                ),
                field_value_span(field, selected),
                Span::styled(hint, Style::default().fg(Color::Yellow)),
            ]));
        }
        FieldKind::Cycle => {
            let hint = if selected { " \u{25C2} \u{25B8}" } else { "" };
            labeled_field(lines, cursor, field, selected, hint, Color::Yellow);
        }
        FieldKind::Toggle => {
            let hint = if selected { "  \u{23CE} toggle" } else { "" };
            labeled_field(lines, cursor, field, selected, hint, Color::DarkGray);
        }
        FieldKind::Number | FieldKind::Text => {
            let hint = if selected { "  \u{23CE} edit" } else { "" };
            labeled_field(lines, cursor, field, selected, hint, Color::DarkGray);
        }
        FieldKind::FilePicker => {
            let hint = if selected { "  \u{23CE} browse" } else { "" };
            lines.push(Line::from(vec![
                Span::raw(cursor),
                Span::styled(
                    format!("{:<9}", field.label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    &field.value_display,
                    Style::default().fg(if selected { Color::Cyan } else { Color::Gray }),
                ),
                Span::styled(hint, Style::default().fg(Color::DarkGray)),
            ]));
        }
        FieldKind::ReadOnly => {
            lines.push(Line::from(vec![
                Span::raw(cursor),
                Span::styled(
                    format!("{:<9}", field.label),
                    Style::default().fg(Color::DarkGray),
                ),
                field_value_span(field, selected),
            ]));
        }
    }
}

fn field_value_span(field: &crate::detail_form::FormField, selected: bool) -> Span<'_> {
    Span::styled(
        &field.value_display,
        Style::default().fg(if selected { Color::White } else { Color::Gray }),
    )
}

fn labeled_field<'a>(
    lines: &mut Vec<Line<'a>>,
    cursor: &str,
    field: &'a crate::detail_form::FormField,
    selected: bool,
    hint: &str,
    hint_color: Color,
) {
    lines.push(Line::from(vec![
        Span::raw(cursor.to_string()),
        Span::styled(
            format!("{:<9}", field.label),
            Style::default().fg(Color::DarkGray),
        ),
        field_value_span(field, selected),
        Span::styled(hint.to_string(), Style::default().fg(hint_color)),
    ]));
}
