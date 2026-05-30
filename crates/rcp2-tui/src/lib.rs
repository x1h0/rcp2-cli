mod app;
mod detail_form;
mod transfer;
mod ui;

use app::{App, MainView};
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use rcp2_core::ops::{TRANSFER_MODE_EMMC, TRANSFER_MODE_SD};
use std::io;
use std::time::Duration;

/// Runs the TUI application.
///
/// # Errors
/// Returns an error if terminal setup, device connection, or event handling fails.
pub fn run(allow_send: bool, accepted: bool) -> Result<(), Box<dyn std::error::Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = std::io::stdout().execute(LeaveAlternateScreen);
        original_hook(info);
    }));

    if !accepted {
        let result = disclaimer_loop(&mut terminal, allow_send);
        match result {
            Ok(false) => {
                terminal::disable_raw_mode()?;
                terminal.backend_mut().execute(LeaveAlternateScreen)?;
                terminal.show_cursor()?;
                return Ok(());
            }
            Err(e) => {
                terminal::disable_raw_mode()?;
                terminal.backend_mut().execute(LeaveAlternateScreen)?;
                terminal.show_cursor()?;
                return Err(e);
            }
            Ok(true) => {}
        }
    }

    terminal.draw(|frame| {
        ui::render_connecting(frame, frame.area());
    })?;

    let mut app = App::connect(allow_send)?;
    let result = run_loop(&mut terminal, &mut app);

    if app.main_view == MainView::Transfer {
        app.leave_transfer_view();
    }

    terminal::disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn disclaimer_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    allow_send: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            ui::render_disclaimer(frame, frame.area(), allow_send);
        })?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('y') => return Ok(true),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
                _ => {}
            }
        }
    }
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        app.poll_device_events();

        if app.modal == app::ModalState::FilePick {
            terminal::disable_raw_mode()?;
            terminal.backend_mut().execute(LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            app.do_file_pick();

            terminal::enable_raw_mode()?;
            terminal.backend_mut().execute(EnterAlternateScreen)?;
            terminal.hide_cursor()?;
            terminal.clear()?;
            continue;
        }

        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && handle_key_press(app, key.code)
        {
            return Ok(());
        }
    }
}

fn handle_key_press(app: &mut App, code: KeyCode) -> bool {
    if app.modal == app::ModalState::Help {
        match code {
            KeyCode::Esc | KeyCode::Char('?' | 'q') => {
                app.modal = app::ModalState::None;
                app.help_scroll = 0;
            }
            KeyCode::Up => app.help_scroll = app.help_scroll.saturating_sub(1),
            KeyCode::Down => app.help_scroll = app.help_scroll.saturating_add(1),
            _ => {}
        }
        return false;
    }

    if handle_modal_key(app, code) {
        return false;
    }

    if app.detail_form.is_some() {
        return handle_detail_form_key(app, code);
    }

    if app.main_view == MainView::Transfer && app.transfer.storage_choice.is_none() {
        match code {
            KeyCode::Char('1') => app.choose_transfer_storage(TRANSFER_MODE_EMMC),
            KeyCode::Char('2') => app.choose_transfer_storage(TRANSFER_MODE_SD),
            KeyCode::Esc => {
                app.main_view = MainView::Pads;
            }
            _ => {}
        }
        return false;
    }

    handle_global_key(app, code)
}

/// Handle keys for modal overlays (confirm dialog, prompts, pad-press wait).
/// Returns `true` if a modal consumed the key (caller should return early).
fn handle_modal_key(app: &mut App, code: KeyCode) -> bool {
    if app.confirm_dialog.is_some() {
        match code {
            KeyCode::Char('y') | KeyCode::Enter => app.confirm_dialog_yes(),
            KeyCode::Char('n') | KeyCode::Esc => app.confirm_dialog_no(),
            _ => {}
        }
        return true;
    }

    if app.has_pad_upload_prompt() {
        match code {
            KeyCode::Enter => app.confirm_pad_upload(),
            KeyCode::Esc => app.cancel_pad_upload(),
            KeyCode::Backspace => app.pad_upload_backspace(),
            KeyCode::Char(c) => app.pad_upload_input(c),
            _ => {}
        }
        return true;
    }

    if app.has_pad_download_prompt() {
        match code {
            KeyCode::Enter => app.confirm_pad_download(),
            KeyCode::Esc => app.cancel_pad_download(),
            KeyCode::Backspace => app.pad_download_backspace(),
            KeyCode::Char(c) => app.pad_download_input(c),
            _ => {}
        }
        return true;
    }

    if app.has_save_prompt() {
        match code {
            KeyCode::Enter => app.transfer_enter(),
            KeyCode::Esc => app.transfer_cancel(),
            KeyCode::Backspace => app.transfer_backspace(),
            KeyCode::Char(c) => app.transfer_input(c),
            _ => {}
        }
        return true;
    }

    if app.modal == app::ModalState::WaitingForPadPress {
        if code == KeyCode::Esc {
            app.modal = app::ModalState::None;
            app.status = "cancelled".into();
        }
        return true;
    }

    false
}

/// Handle keys when the detail form is open. Returns `true` to quit.
fn handle_detail_form_key(app: &mut App, code: KeyCode) -> bool {
    if app
        .detail_form
        .as_ref()
        .is_some_and(detail_form::DetailForm::is_editing)
    {
        match code {
            KeyCode::Enter => app.detail_form_enter(),
            KeyCode::Esc => {
                if let Some(ref mut f) = app.detail_form {
                    f.cancel_text_edit();
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut f) = app.detail_form {
                    f.edit_backspace();
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut f) = app.detail_form {
                    f.edit_type_char(c);
                }
            }
            _ => {}
        }
        return false;
    }

    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {
            if app.detail_form.as_ref().is_some_and(|f| f.is_replace) {
                app.open_detail_form();
            } else {
                app.close_detail_form();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(ref mut f) = app.detail_form {
                f.move_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ref mut f) = app.detail_form {
                f.move_down();
            }
        }
        KeyCode::Enter => app.detail_form_enter(),
        KeyCode::Left | KeyCode::Char('h') => app.detail_form_left(),
        KeyCode::Right | KeyCode::Char('l') => app.detail_form_right(),
        _ => {}
    }
    false
}

/// Handle global keys (no modal/form active). Returns `true` to quit.
fn handle_global_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('?') => app.modal = app::ModalState::Help,
        KeyCode::Esc => match app.main_view {
            MainView::Transfer => app.transfer_cancel(),
            MainView::Monitor => app.toggle_main_view(),
            MainView::Pads => {}
        },
        KeyCode::Char('t') if app.main_view == MainView::Pads => app.enter_transfer_view(),
        KeyCode::Char('m') if app.main_view != MainView::Transfer => app.toggle_main_view(),
        KeyCode::Char('s') if app.main_view == MainView::Monitor => {
            if let Err(e) = app.save_log() {
                app.status = format!("save failed: {e}");
            }
        }
        KeyCode::Left | KeyCode::Char('h') => app.prev_bank(),
        KeyCode::Right | KeyCode::Char('l') => app.next_bank(),
        KeyCode::Up | KeyCode::Char('k') => match app.main_view {
            MainView::Monitor => app.scroll_log_up(),
            MainView::Transfer => app.transfer_select_up(),
            MainView::Pads => app.prev_pad(),
        },
        KeyCode::Down | KeyCode::Char('j') => match app.main_view {
            MainView::Monitor => app.scroll_log_down(),
            MainView::Transfer => app.transfer_select_down(),
            MainView::Pads => app.next_pad(),
        },
        KeyCode::Char(c @ '1'..='8') if app.main_view == MainView::Pads => {
            app.select_pad((c as usize) - ('1' as usize));
        }
        KeyCode::Char('r') if app.allow_send => app.toggle_recording(),
        KeyCode::Char('R') if app.allow_send => app.stop_recording(),
        KeyCode::Char('p') if app.main_view == MainView::Pads && app.allow_send => {
            app.tap_pad();
        }
        KeyCode::Enter if app.main_view == MainView::Pads && app.allow_send => {
            app.open_detail_form();
        }
        KeyCode::Enter if app.main_view == MainView::Transfer => {
            app.transfer_enter();
        }
        KeyCode::Char('d') if app.main_view == MainView::Transfer => {
            app.transfer_download_selected();
        }
        _ => {}
    }
    false
}
