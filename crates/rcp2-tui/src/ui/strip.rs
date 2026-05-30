use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use rcp2_core::{PHYSICAL_FADERS, VIRTUAL_FADERS};

use super::util::level_bar;

pub(super) fn render_device_strip(frame: &mut Frame, area: Rect, app: &App) {
    let vm = &app.vm;

    let pot_count = vm.pots.len().min(4);
    let physical = PHYSICAL_FADERS.min(vm.faders.len());
    let virtual_count = VIRTUAL_FADERS.min(vm.faders.len().saturating_sub(PHYSICAL_FADERS));

    let has_pots = pot_count > 0;
    let has_physical = physical > 0;
    let has_virtual = virtual_count > 0;

    let mut sections: Vec<Constraint> = Vec::new();
    if has_pots {
        sections.push(Constraint::Ratio(
            pot_count as u32,
            (pot_count + physical + virtual_count) as u32,
        ));
    }
    if has_physical {
        sections.push(Constraint::Ratio(
            physical as u32,
            (pot_count + physical + virtual_count) as u32,
        ));
    }
    if has_virtual {
        sections.push(Constraint::Ratio(
            virtual_count as u32,
            (pot_count + physical + virtual_count) as u32,
        ));
    }

    let section_areas = Layout::horizontal(sections).split(area);
    let mut sec = 0;

    if has_pots {
        render_pot_section(frame, section_areas[sec], &vm.pots[..pot_count]);
        sec += 1;
    }

    if has_physical {
        render_fader_section(
            frame,
            section_areas[sec],
            &vm.faders[..physical],
            "Faders",
            1,
        );
        sec += 1;
    }

    if has_virtual {
        render_virtual_section(
            frame,
            section_areas[sec],
            &vm.faders[PHYSICAL_FADERS..PHYSICAL_FADERS + virtual_count],
        );
    }
}

fn render_pot_section(frame: &mut Frame, area: Rect, pots: &[u32]) {
    let block = Block::default()
        .title(" Pots ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let constraints: Vec<Constraint> = pots
        .iter()
        .map(|_| Constraint::Ratio(1, pots.len() as u32))
        .collect();
    let cols = Layout::horizontal(constraints).split(inner);

    for (i, &level) in pots.iter().enumerate() {
        let pct = level * 100 / 127;
        let bar = level_bar(f64::from(level), 127.0);
        render_channel(
            frame,
            cols[i],
            &format!("P{}", i + 1),
            &bar,
            Some(pct),
            Color::Magenta,
        );
    }
}

fn render_fader_section(
    frame: &mut Frame,
    area: Rect,
    faders: &[rcp2_core::FaderInfo],
    title: &str,
    start_num: usize,
) {
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let constraints: Vec<Constraint> = faders
        .iter()
        .map(|_| Constraint::Ratio(1, faders.len() as u32))
        .collect();
    let cols = Layout::horizontal(constraints).split(inner);

    for (i, fader) in faders.iter().enumerate() {
        let pct = (fader.percent() * 100.0).round() as u32;
        let bar = level_bar(fader.percent(), 1.0);
        let color = fader_color(fader.mute, fader.cue);
        let suffix = fader_suffix(fader.mute, fader.cue);
        let label = format!("F{}{suffix}", start_num + i);
        render_channel(frame, cols[i], &label, &bar, Some(pct), color);
    }
}

fn render_virtual_section(frame: &mut Frame, area: Rect, faders: &[rcp2_core::FaderInfo]) {
    let block = Block::default()
        .title(" Virtual ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let constraints: Vec<Constraint> = faders
        .iter()
        .map(|_| Constraint::Ratio(1, faders.len() as u32))
        .collect();
    let cols = Layout::horizontal(constraints).split(inner);

    for (i, fader) in faders.iter().enumerate() {
        let label = format!("V{}", i + 1);
        if fader.configured {
            let pct = (fader.percent() * 100.0).round() as u32;
            let bar = level_bar(fader.percent(), 1.0);
            let color = fader_color(fader.mute, fader.cue);
            render_channel(frame, cols[i], &label, &bar, Some(pct), color);
        } else {
            render_channel(frame, cols[i], &label, "", None, Color::DarkGray);
        }
    }
}

fn fader_color(muted: bool, soloed: bool) -> Color {
    if muted {
        Color::Red
    } else if soloed {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn fader_suffix(muted: bool, soloed: bool) -> &'static str {
    if muted {
        " M"
    } else if soloed {
        " S"
    } else {
        ""
    }
}

fn render_channel(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    bar: &str,
    pct: Option<u32>,
    color: Color,
) {
    let w = area.width as usize;

    let line2 = if let Some(p) = pct {
        let pct_str = format!("{p}%");
        let need_full = 1 + 4 + 1 + pct_str.len();
        if w >= need_full {
            Line::from(vec![
                Span::styled(format!(" {bar} "), Style::default().fg(color)),
                Span::styled(pct_str, Style::default().fg(Color::DarkGray)),
            ])
        } else if w >= 5 {
            Line::from(Span::styled(format!(" {bar}"), Style::default().fg(color)))
        } else {
            Line::raw("")
        }
    } else if w >= 4 {
        Line::styled(" --", Style::default().fg(Color::DarkGray))
    } else {
        Line::raw("")
    };

    let text = vec![
        Line::from(Span::styled(
            format!(" {label}"),
            Style::default().fg(color),
        )),
        line2,
    ];
    frame.render_widget(Paragraph::new(text), area);
}
