use super::{App, ConfirmAction, ConfirmDialog, MoveSelection, PadHold};
use crate::detail_form::{DetailForm, FieldKind};
use crate::transfer::{PadMove, PadMoveStatus, PadUpload, PadUploadStatus};
use log::info;
use rcp2_core::ops::TRANSFER_MODE_EMMC;
use rcp2_core::ops::pad as pad_ops;
use rcp2_protocol::types::Value;
use std::time::Instant;

impl App {
    pub fn tap_pad(&mut self) {
        if let Err(e) = pad_ops::tap_pad(&self.conn, self.logical_pad_position(), self.profile) {
            self.status = format!("trigger failed: {e}");
        }
    }

    pub fn trigger_pad(&mut self) {
        match self.pad_hold {
            PadHold::Tap => self.tap_pad(),
            PadHold::Held(..) => {}
            PadHold::Idle => {
                let position = self.logical_pad_position();
                if let Err(e) = pad_ops::press_pad(&self.conn, position, self.profile) {
                    self.status = format!("trigger failed: {e}");
                    return;
                }
                self.pad_hold = PadHold::Held(position, Instant::now());
            }
        }
    }

    pub fn pad_release(&mut self) {
        let PadHold::Held(position, start) = self.pad_hold else {
            return;
        };
        let elapsed = start.elapsed();
        if elapsed < pad_ops::DEFAULT_PRESS {
            std::thread::sleep(pad_ops::DEFAULT_PRESS.saturating_sub(elapsed));
        }
        if let Err(e) = pad_ops::release_pad(&self.conn, position, self.profile) {
            self.status = format!("trigger failed: {e}");
        }
        self.pad_hold = PadHold::Idle;
    }

    pub(super) fn send_pad_property(&mut self, name: &str, value: Value) {
        let Some(sp_idx) = self.vm.soundpads_idx else {
            return;
        };
        let Some(pad) = self.selected_pad_info() else {
            return;
        };
        let child_idx = pad.child_index;
        if let Err(e) = pad_ops::send_property(&self.conn, sp_idx, child_idx, name, value) {
            self.status = format!("failed: {e}");
            return;
        }
        if let Ok(state) = self.conn.state().snapshot() {
            self.vm.refresh(&state);
            self.refresh_detail_form();
        }
    }

    pub(super) fn cycle_field(&mut self, prop: &str, forward: bool) {
        use crate::detail_form::play_mode_count;

        let Some(pad) = self.selected_pad_info() else {
            return;
        };
        if prop == "padPlayMode" {
            let count = play_mode_count();
            let current = pad.play_mode;
            let new_val = if forward {
                (current + 1) % count
            } else {
                (current + count - 1) % count
            };
            self.send_pad_property("padPlayMode", Value::U32(new_val));
        }
    }

    pub fn open_detail_form(&mut self) {
        let Some(pad) = self.selected_pad_info() else {
            if self.dry_run {
                self.status = "pad creation disabled in dry-run".into();
                return;
            }
            let pad_idx =
                self.vm.selected_bank * self.profile.pads_per_bank + self.logical_pad_position();
            self.detail_form = Some(DetailForm::new_pad(pad_idx));
            return;
        };
        self.detail_form = Some(DetailForm::from_pad(pad, self.dry_run));
    }

    pub fn detail_form_enter(&mut self) {
        let Some(ref mut form) = self.detail_form else {
            return;
        };

        if form.is_editing() {
            let kind = form.fields.get(form.selected).map(|f| f.kind);
            let result = form.finish_text_edit();
            let prop = form
                .fields
                .get(form.selected)
                .and_then(|f| f.property.clone());
            if let (Some(new_value), Some(prop)) = (result, prop) {
                match kind {
                    Some(FieldKind::Number) => {
                        if let Ok(v) = new_value.parse::<f64>() {
                            self.send_pad_property(&prop, Value::F64(v));
                        } else {
                            self.status = "invalid number".into();
                        }
                    }
                    _ => self.send_pad_property(&prop, Value::String(new_value)),
                }
            }
            return;
        }

        let field = form.selected_field().cloned();
        let Some(field) = field else { return };

        match field.kind {
            FieldKind::Text | FieldKind::Number => {
                if let Some(ref mut f) = self.detail_form {
                    f.start_text_edit();
                }
            }
            FieldKind::FilePicker => {
                self.open_file_picker();
            }
            FieldKind::Toggle => {
                if let Some(prop) = field.property {
                    let new_val = field.value_display != "Yes";
                    self.send_pad_property(&prop, Value::Bool(new_val));
                }
            }
            FieldKind::Action => {
                if let Some(prop) = field.property {
                    match prop.as_str() {
                        "download" => self.start_pad_download(),
                        "upload" if self.dry_run => {
                            self.status = "upload disabled in dry-run".into();
                        }
                        "upload" => self.open_file_picker(),
                        "play" => self.tap_pad(),
                        "create" => self.confirm_create_pad(),
                        "replaceConfirm" => self.confirm_replace_sound(),
                        "replaceCancel" => self.open_detail_form(),
                        "move" => self.start_move_target(),
                        "delete" => self.delete_pad(),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    pub fn detail_form_left(&mut self) {
        self.detail_form_cycle(false);
    }

    pub fn detail_form_right(&mut self) {
        self.detail_form_cycle(true);
    }

    fn detail_form_cycle(&mut self, forward: bool) {
        let Some(ref mut form) = self.detail_form else {
            return;
        };
        let Some(field) = form.selected_field() else {
            return;
        };
        if field.kind == FieldKind::Cycle {
            if let Some(ref prop) = field.property.clone() {
                self.cycle_field(prop, forward);
            }
            return;
        }
        if field.kind != FieldKind::ColorCycle {
            return;
        }
        let offset = if forward { 1 } else { 11 };
        if form.new_pad_idx.is_some() {
            form.new_pad_color = (form.new_pad_color + offset) % 12;
            let (r, g, b) = rcp2_core::PadColor::from_index(form.new_pad_color).to_rgb();
            if let Some(f) = form
                .fields
                .iter_mut()
                .find(|f| f.property.as_deref() == Some("padColourIndex"))
            {
                f.value_display = format!("#{r:02x}{g:02x}{b:02x}");
            }
        } else {
            let pad = self.selected_pad_info().cloned();
            if let Some(pad) = pad {
                let new_color = (pad.color_index + offset) % 12;
                self.send_pad_property(
                    "padColourIndex",
                    Value::U32(u32::try_from(new_color).unwrap_or(0)),
                );
            }
        }
    }

    pub fn close_detail_form(&mut self) {
        self.detail_form = None;
    }

    pub(super) fn refresh_detail_form(&mut self) {
        if self.detail_form.is_none() {
            return;
        }
        if let Some(ref form) = self.detail_form
            && form.is_editing()
        {
            return;
        }
        let pad = self.selected_pad_info().cloned();
        let Some(pad) = pad else { return };
        let dry_run = self.dry_run;
        if let Some(ref mut form) = self.detail_form {
            let selected = form.selected;
            *form = DetailForm::from_pad(&pad, dry_run);
            form.selected = selected.min(form.fields.len().saturating_sub(1));
        }
    }

    fn open_file_picker(&mut self) {
        self.modal = super::ModalState::FilePick;
    }

    pub fn do_file_pick(&mut self) {
        self.modal = super::ModalState::None;

        let dialog = rfd::FileDialog::new()
            .set_title("Select sound file")
            .add_filter("Audio (WAV/MP3)", &["wav", "mp3"]);

        let Some(path) = dialog.pick_file() else {
            return;
        };

        let path_str = path.to_string_lossy().into_owned();
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let pad = self.selected_pad_info().cloned();

        let Some(ref mut form) = self.detail_form else {
            return;
        };

        let field = form.selected_field().cloned();
        if let Some(field) = field
            && let Some(ref prop) = field.property
        {
            match prop.as_str() {
                "soundFile" => {
                    if let Some(f) = form
                        .fields
                        .iter_mut()
                        .find(|f| f.property.as_deref() == Some("soundFile"))
                    {
                        f.value_display = filename;
                    }
                    form.picked_file_path = Some(path_str.clone());

                    let duration =
                        rcp2_core::ops::audio::audio_duration_secs(std::path::Path::new(&path_str));
                    form.audio_duration = duration;

                    if let Some(dur) = duration {
                        use crate::detail_form::{FieldKind, FormField};

                        form.fields.retain(|f| {
                            !matches!(
                                f.property.as_deref(),
                                Some("padDuration" | "padEnvStart" | "padEnvStop")
                            )
                        });
                        let insert_pos = form
                            .fields
                            .iter()
                            .position(|f| f.property.as_deref() == Some("create"))
                            .unwrap_or(form.fields.len());
                        let new_fields = [
                            FormField {
                                label: "Duration".into(),
                                kind: FieldKind::ReadOnly,
                                value_display: format!("{dur:.2}s"),
                                property: Some("padDuration".into()),
                            },
                            FormField {
                                label: "Start".into(),
                                kind: FieldKind::Number,
                                value_display: "0.0".into(),
                                property: Some("padEnvStart".into()),
                            },
                            FormField {
                                label: "End".into(),
                                kind: FieldKind::Number,
                                value_display: format!("{dur:.2}"),
                                property: Some("padEnvStop".into()),
                            },
                        ];
                        for (i, field) in new_fields.into_iter().enumerate() {
                            form.fields.insert(insert_pos + i, field);
                        }
                    }
                }
                "upload" => {
                    let duration =
                        rcp2_core::ops::audio::audio_duration_secs(std::path::Path::new(&path_str));

                    let pad_name = pad.as_ref().map(|p| p.name.clone()).unwrap_or_default();
                    let replace_form =
                        DetailForm::replace_sound(&pad_name, &filename, path_str.clone(), duration);
                    self.detail_form = Some(replace_form);
                }
                _ => {}
            }
        }
    }

    fn confirm_replace_sound(&mut self) {
        if self.dry_run {
            self.status = "replace disabled in dry-run".into();
            return;
        }
        if !self.require_no_active_download() {
            return;
        }
        let Some(ref form) = self.detail_form else {
            return;
        };
        let Some(ref path) = form.picked_file_path else {
            self.status = "select a sound file first".into();
            return;
        };
        if !std::path::Path::new(path).exists() {
            self.status = format!("file not found: {path}");
            return;
        }

        let pad_name = self
            .selected_pad_info()
            .map(|p| p.name.clone())
            .unwrap_or_default();

        let duration = form.audio_duration;
        let env_start = form
            .fields
            .iter()
            .find(|f| f.property.as_deref() == Some("replaceEnvStart"))
            .and_then(|f| f.value_display.parse::<f64>().ok());
        let env_stop = form
            .fields
            .iter()
            .find(|f| f.property.as_deref() == Some("replaceEnvStop"))
            .and_then(|f| f.value_display.parse::<f64>().ok());

        self.confirm_dialog = Some(ConfirmDialog {
            title: "Replace Sound".into(),
            message: format!(
                "Replace sound on \"{pad_name}\"?\nThe current sound will be overwritten.\n\n⚠ Transfer mode will be activated.\n⚠ Device audio & pads will be unavailable."
            ),
            action: ConfirmAction::ReplaceSound {
                path: path.clone(),
                env_start,
                env_stop,
                duration,
            },
        });
    }

    pub(super) fn start_pad_replace_with_env(
        &mut self,
        path: &str,
        env_start: Option<f64>,
        env_stop: Option<f64>,
        duration: Option<f64>,
    ) {
        if !self.require_transfer_tools() {
            return;
        }
        if !self.require_no_active_download() {
            return;
        }
        let ext = match pad_ops::validate_audio_file(path) {
            Ok(ext) => ext,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let Some(pad) = self.selected_pad_info() else {
            return;
        };
        let Some(sp_idx) = self.vm.soundpads_idx else {
            return;
        };

        let (env_start_norm, env_stop_norm) = match (duration, env_start, env_stop) {
            (Some(dur), Some(start), Some(stop)) if dur > 0.0 => {
                (Some(start / dur), Some(stop / dur))
            }
            _ => (None, None),
        };

        let upload = PadUpload::prepare_replace(
            &self.conn,
            sp_idx,
            pad.child_index,
            pad.idx,
            path,
            &ext,
            (env_start_norm, env_stop_norm),
        );
        self.pad_upload = Some(upload);
        self.detail_form = None;
        self.activate_transfer_mode(TRANSFER_MODE_EMMC);
        self.status = "uploading sound...".into();
    }

    fn confirm_create_pad(&mut self) {
        if self.dry_run {
            self.status = "pad creation disabled in dry-run".into();
            return;
        }
        if !self.require_transfer_tools() {
            return;
        }
        if !self.require_no_active_download() {
            return;
        }
        let Some(ref form) = self.detail_form else {
            return;
        };
        if form.new_pad_idx.is_none() {
            return;
        }
        let sound_file = form.picked_file_path.clone().unwrap_or_default();
        if sound_file.is_empty() {
            self.status = "select a sound file first".into();
            return;
        }
        if !std::path::Path::new(&sound_file).exists() {
            self.status = format!("file not found: {sound_file}");
            return;
        }
        self.confirm_dialog = Some(ConfirmDialog {
            title: "Create Pad".into(),
            message: "This will upload the sound file to the device.\n\n⚠ Transfer mode will be activated.\n⚠ Device audio & pads will be unavailable.".into(),
            action: ConfirmAction::CreatePad,
        });
    }

    pub(super) fn create_new_pad(&mut self) {
        let Some(ref form) = self.detail_form else {
            return;
        };
        let Some(pad_idx) = form.new_pad_idx else {
            return;
        };

        let sound_file = form.picked_file_path.clone().unwrap_or_default();
        if let Err(e) = pad_ops::validate_audio_file(&sound_file) {
            self.status = e;
            return;
        }

        let form_name = form
            .fields
            .iter()
            .find(|f| f.property.as_deref() == Some("padName"))
            .map(|f| f.value_display.clone())
            .unwrap_or_default();

        let pad_name = if form_name.is_empty() {
            std::path::Path::new(&sound_file).file_stem().map_or_else(
                || format!("Pad {}", pad_idx + 1),
                |n| n.to_string_lossy().into_owned(),
            )
        } else {
            form_name
        };

        let duration = form.audio_duration;
        let env_start = form
            .fields
            .iter()
            .find(|f| f.property.as_deref() == Some("padEnvStart"))
            .and_then(|f| f.value_display.parse::<f64>().ok());
        let env_stop = form
            .fields
            .iter()
            .find(|f| f.property.as_deref() == Some("padEnvStop"))
            .and_then(|f| f.value_display.parse::<f64>().ok());

        let (env_start, env_stop) = match (duration, env_start, env_stop) {
            (Some(dur), Some(start), Some(stop)) if dur > 0.0 => {
                (Some(start / dur), Some(stop / dur))
            }
            _ => (None, None),
        };

        info!("creating pad {pad_idx} ({pad_name}) with sound: {sound_file}");
        self.detail_form = None;

        let upload = PadUpload::start_new(pad_idx, sound_file, pad_name, env_start, env_stop);
        self.pad_upload = Some(upload);
        self.activate_transfer_mode(TRANSFER_MODE_EMMC);
        self.status = "uploading sound...".into();
    }

    fn delete_pad(&mut self) {
        let Some(pad) = self.selected_pad_info() else {
            return;
        };
        self.confirm_dialog = Some(ConfirmDialog {
            title: "Delete Pad".into(),
            message: format!("Delete \"{}\"? This cannot be undone.", pad.name),
            action: ConfirmAction::DeletePad,
        });
    }

    pub(super) fn confirm_delete_pad(&mut self) {
        let Some(pad) = self.selected_pad_info().cloned() else {
            return;
        };
        let Some(sp_idx) = self.vm.soundpads_idx else {
            return;
        };
        match pad_ops::delete_pad(&self.conn, sp_idx, pad.child_index) {
            Ok(()) => {
                self.status = format!("deleted pad: {}", pad.name);
                self.detail_form = None;
                self.request_full_state();
            }
            Err(e) => self.status = format!("delete failed: {e}"),
        }
    }

    fn start_move_target(&mut self) {
        if self.dry_run {
            self.status = "move disabled in dry-run".into();
            return;
        }
        let Some(pad) = self.selected_pad_info().cloned() else {
            return;
        };
        self.detail_form = None;
        if !self.snap_move_cursor_to_empty() {
            self.status = "no empty slot available".into();
            return;
        }
        self.move_selection = Some(MoveSelection {
            src_idx: pad.idx,
            src_child_index: pad.child_index,
            name: pad.name.clone(),
        });
        self.status = format!(
            "move \"{}\": select empty target slot (Enter confirm, Esc cancel)",
            pad.name
        );
    }

    fn slot_is_empty(&self, bank: usize, display_pos: usize) -> bool {
        let logical = rcp2_core::BankView::logical_index(display_pos, self.profile);
        let idx = bank * self.profile.pads_per_bank + logical;
        !self.vm.pads.iter().any(|p| p.idx == idx)
    }

    fn first_empty_in_bank(&self, bank: usize) -> Option<usize> {
        (0..self.profile.pads_per_bank).find(|&pos| self.slot_is_empty(bank, pos))
    }

    fn snap_move_cursor_to_empty(&mut self) -> bool {
        if let Some(pos) = self.first_empty_in_bank(self.vm.selected_bank) {
            self.selected_pad = pos;
            return true;
        }
        for b in 0..self.vm.bank_count() {
            if let Some(pos) = self.first_empty_in_bank(b) {
                self.vm.selected_bank = b;
                self.sync_bank_to_device();
                self.selected_pad = pos;
                return true;
            }
        }
        false
    }

    pub fn move_target_next_pad(&mut self) {
        let ppb = self.profile.pads_per_bank;
        for step in 1..=ppb {
            let pos = (self.selected_pad + step) % ppb;
            if self.slot_is_empty(self.vm.selected_bank, pos) {
                self.selected_pad = pos;
                return;
            }
        }
    }

    pub fn move_target_prev_pad(&mut self) {
        let ppb = self.profile.pads_per_bank;
        for step in 1..=ppb {
            let pos = (self.selected_pad + ppb - step) % ppb;
            if self.slot_is_empty(self.vm.selected_bank, pos) {
                self.selected_pad = pos;
                return;
            }
        }
    }

    pub fn move_target_next_bank(&mut self) {
        let count = self.vm.bank_count();
        if count == 0 {
            return;
        }
        self.vm.selected_bank = (self.vm.selected_bank + 1) % count;
        self.sync_bank_to_device();
        self.selected_pad = self.first_empty_in_bank(self.vm.selected_bank).unwrap_or(0);
    }

    pub fn move_target_prev_bank(&mut self) {
        let count = self.vm.bank_count();
        if count == 0 {
            return;
        }
        self.vm.selected_bank = (self.vm.selected_bank + count - 1) % count;
        self.sync_bank_to_device();
        self.selected_pad = self.first_empty_in_bank(self.vm.selected_bank).unwrap_or(0);
    }

    pub fn move_target_select(&mut self, pos: usize) {
        if pos < self.profile.pads_per_bank && self.slot_is_empty(self.vm.selected_bank, pos) {
            self.selected_pad = pos;
        }
    }

    pub fn confirm_move_target(&mut self) {
        let Some(sel) = self.move_selection.clone() else {
            return;
        };
        let position = self.logical_pad_position();
        let dst_idx = self.vm.selected_bank * self.profile.pads_per_bank + position;
        if dst_idx == sel.src_idx {
            self.status = "same slot — pick another".into();
            return;
        }
        if self.vm.pads.iter().any(|p| p.idx == dst_idx) {
            self.status = "slot occupied — pick an empty slot".into();
            return;
        }
        self.confirm_dialog = Some(ConfirmDialog {
            title: "Move Pad".into(),
            message: format!(
                "Move \"{}\" to bank {} slot {}?\n\n⚠ Transfer mode will be activated.\n⚠ Device audio & pads will be unavailable.",
                sel.name,
                self.vm.selected_bank + 1,
                position + 1
            ),
            action: ConfirmAction::MovePad { dst_idx },
        });
    }

    pub fn cancel_move_target(&mut self) {
        if self.move_selection.take().is_some() {
            self.status = "move cancelled".into();
        }
    }

    pub(super) fn start_move_pad_transfer(&mut self, dst_idx: usize) {
        let Some(sel) = self.move_selection.take() else {
            return;
        };
        if !self.require_no_active_download() {
            return;
        }
        if self.pad_upload.is_some() {
            self.status = "upload in progress".into();
            return;
        }
        let Some(sp_idx) = self.vm.soundpads_idx else {
            return;
        };
        let snapshot = match self.conn.state().snapshot() {
            Ok(s) => s,
            Err(e) => {
                self.status = format!("move failed: {e}");
                return;
            }
        };
        let Some(node) = snapshot
            .children
            .get(sp_idx)
            .and_then(|sp| sp.children.get(sel.src_child_index))
        else {
            self.status = "move failed: source pad not found".into();
            return;
        };

        let mut props = node.properties.clone();
        props.insert("padActive".into(), Value::Bool(false));
        props.insert("padProgress".into(), Value::F64(0.0));

        let filename = props.get("padFilePath").and_then(|v| match v {
            Value::String(p) if !p.is_empty() => std::path::Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned()),
            _ => None,
        });
        let has_file = filename.is_some();
        if has_file && !self.require_transfer_tools() {
            return;
        }

        self.pad_move = Some(PadMove::new(
            sel.src_idx,
            sel.src_child_index,
            dst_idx,
            props,
            filename,
            sel.name,
        ));
        if has_file {
            self.activate_transfer_mode(TRANSFER_MODE_EMMC);
        }
        self.status = "moving pad...".into();
    }

    pub(super) fn poll_pad_move(&mut self) {
        let Some(ref mut mv) = self.pad_move else {
            return;
        };
        match mv.poll(&self.conn, &mut self.transfer, &self.vm) {
            PadMoveStatus::InProgress(msg) => self.status = msg,
            PadMoveStatus::Done(msg) => {
                self.status = msg;
                self.pad_move = None;
                self.request_full_state();
            }
            PadMoveStatus::Error(msg) => {
                self.status = msg;
                self.pad_move = None;
            }
        }
    }

    pub(super) fn poll_pad_upload(&mut self) {
        let Some(ref mut ul) = self.pad_upload else {
            return;
        };
        match ul.poll(&self.conn, &mut self.transfer, &self.vm) {
            PadUploadStatus::InProgress(msg) => self.status = msg,
            PadUploadStatus::Done(msg) => {
                self.status = msg;
                self.pad_upload = None;
                self.request_full_state();
            }
            PadUploadStatus::Error(msg) => {
                self.status = msg;
                self.pad_upload = None;
            }
        }
    }
}
