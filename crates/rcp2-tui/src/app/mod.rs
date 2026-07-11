mod pad_ops;
mod transfer_ops;

use crate::detail_form::DetailForm;
use crate::transfer::{PadDownload, PadMove, PadUpload, TransferState, TransferStatus};
use log::{info, warn};
use rcp2_core::recording::RecordingTimer;
use rcp2_core::{BankView, DeviceProfile, DeviceViewModel, PadInfo, RecordingStatus};
use rcp2_protocol::device::{DeviceConnection, DeviceEvent};
use rcp2_protocol::transport::hid::HidTransport;
use rcp2_protocol::types::Value;
use std::collections::VecDeque;
use std::io::Write;
use std::time::Instant;

const MAX_LOG_ENTRIES: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainView {
    Pads,
    Monitor,
    Transfer,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsFocus {
    Categories,
    Items,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BtDisconnect {
    Idle,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SerialVisibility {
    Hidden,
    Shown,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsDraft {
    pub lang: Option<usize>,
    pub timezone: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeletePad,
    ReplaceSound {
        path: String,
        env_start: Option<f64>,
        env_stop: Option<f64>,
        duration: Option<f64>,
    },
    CreatePad,
    MovePad {
        dst_idx: usize,
    },
}

#[derive(Debug, Clone)]
pub struct MoveSelection {
    pub src_idx: usize,
    pub src_child_index: usize,
    pub name: String,
}

pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModalState {
    None,
    Help,
    FilePick,
    WaitingForPadPress,
}

pub struct App {
    pub(super) conn: DeviceConnection,
    pub profile: &'static DeviceProfile,
    pub vm: DeviceViewModel,
    pub selected_pad: usize,
    pub detail_scroll: u16,
    pub settings_selected: usize,
    pub settings_item: usize,
    pub settings_focus: SettingsFocus,
    pub settings_scroll: u16,
    pub settings_draft: SettingsDraft,
    pub settings_bt_disconnect: BtDisconnect,
    pub settings_serial: SerialVisibility,
    pub status: String,
    pub connected: bool,
    pub main_view: MainView,
    pub event_log: VecDeque<String>,
    pub log_total: usize,
    pub log_scroll: usize,
    pub help_scroll: u16,
    pub help_max_scroll: u16,
    pub dry_run: bool,
    pub has_transfer_tools: bool,
    pub detail_form: Option<DetailForm>,
    pub modal: ModalState,
    pub transfer: TransferState,
    pub pad_download: Option<PadDownload>,
    pub pad_upload: Option<PadUpload>,
    pub pad_move: Option<PadMove>,
    pub move_selection: Option<MoveSelection>,
    pub confirm_dialog: Option<ConfirmDialog>,
    pub(super) rec_timer: RecordingTimer,
    pub(super) log_start: Instant,
    pub(super) pad_hold: PadHold,
}

#[derive(Clone, Copy)]
pub(crate) enum PadHold {
    Tap,
    Idle,
    Held(usize, Instant),
}

impl App {
    pub fn connect(dry_run: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let hid_api = hidapi::HidApi::new()?;
        let ((rx, tx), model) = HidTransport::open_pair(&hid_api)?;
        let conn = DeviceConnection::open(Box::new(rx), Box::new(tx), model, dry_run)?;
        let profile = model.profile();

        info!("waiting for device state...");
        conn.wait_for_state()?;

        let state = conn.state().snapshot()?;
        let vm = DeviceViewModel::from_state(&state, profile);

        info!(
            "connected to {model}: {} pads, {} faders, FW {}",
            vm.pads.len(),
            vm.faders.len(),
            vm.system.firmware
        );

        Ok(App {
            conn,
            profile,
            vm,
            selected_pad: 0,
            detail_scroll: 0,
            settings_selected: 0,
            settings_item: 0,
            settings_focus: SettingsFocus::Categories,
            settings_scroll: 0,
            settings_draft: SettingsDraft::default(),
            settings_bt_disconnect: BtDisconnect::Idle,
            settings_serial: SerialVisibility::Hidden,
            status: if dry_run {
                "connected (dry-run)".into()
            } else {
                "connected".into()
            },
            connected: true,
            main_view: MainView::Pads,
            event_log: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            log_total: 0,
            log_scroll: 0,
            help_scroll: 0,
            help_max_scroll: 0,
            dry_run,
            has_transfer_tools: rcp2_core::ops::transfer::tools_available(),
            detail_form: None,
            modal: ModalState::None,
            transfer: TransferState::new(),
            pad_download: None,
            pad_upload: None,
            pad_move: None,
            move_selection: None,
            confirm_dialog: None,
            rec_timer: RecordingTimer::new(),
            log_start: Instant::now(),
            pad_hold: PadHold::Tap,
        })
    }

    pub fn poll_device_events(&mut self) {
        let mut changed = false;
        while let Ok(event) = self.conn.events().try_recv() {
            match &event {
                DeviceEvent::PropertyUpdated {
                    indices,
                    name,
                    value,
                } => {
                    changed = true;
                    self.push_log(&format!("[update] {indices:?} {name} = {value:?}"));

                    if self.modal == ModalState::WaitingForPadPress
                        && name == "padButtonPressed"
                        && *value == Value::Bool(true)
                        && indices.len() == 2
                        && indices[0] == rcp2_protocol::device::PHYSICAL_INTERFACE_IDX
                    {
                        let button_pos = indices[1].saturating_sub(self.profile.padbutton_offset);
                        let pad_idx =
                            self.vm.selected_bank * self.profile.pads_per_bank + button_pos;
                        self.modal = ModalState::None;
                        self.detail_form = Some(DetailForm::new_pad(pad_idx));
                        self.status = format!(
                            "configuring pad at bank {} position {}",
                            self.vm.selected_bank + 1,
                            button_pos + 1
                        );
                    }
                }
                DeviceEvent::StateInitialized => {
                    changed = true;
                    self.status = "state refreshed".into();
                    self.push_log("[state] full state received");
                }
                DeviceEvent::UnknownPacket(data) => {
                    self.push_log(&format!(
                        "[unknown] {} bytes: {:02x?}",
                        data.len(),
                        &data[..data.len().min(32)]
                    ));
                    if matches!(data.first(), Some(0x03 | 0x04)) {
                        self.request_full_state();
                    }
                }
                DeviceEvent::Disconnected => {
                    self.connected = false;
                    self.status = "disconnected".into();
                    self.push_log("[disconnected]");
                    self.rec_timer.reset();
                }
                DeviceEvent::Error(e) => {
                    self.status = format!("error: {e}");
                    self.push_log(&format!("[error] {e}"));
                }
            }
        }
        if changed && let Ok(state) = self.conn.state().snapshot() {
            let prev_state = self.vm.recorder.state;
            self.vm.refresh(&state);
            self.update_rec_timer(prev_state);
            self.refresh_detail_form();
            if self.settings_bt_disconnect == BtDisconnect::Pending
                && self.vm.network.bluetooth.connected.is_empty()
            {
                self.settings_bt_disconnect = BtDisconnect::Idle;
                self.settings_focus = SettingsFocus::Categories;
                self.status = "bluetooth disconnected".into();
            }
        }

        self.poll_pad_download();
        self.poll_pad_upload();
        self.poll_pad_move();

        if self.transfer.is_copying() {
            self.transfer.poll_copy();
            self.status.clone_from(&self.transfer.message);
        }

        if self.transfer.status == TransferStatus::Activating && self.transfer.find_mount_point() {
            self.transfer.status = TransferStatus::Active;
            self.transfer.refresh_files();
            self.status = format!(
                "transfer mode active: {}",
                self.transfer.mount_point.as_deref().unwrap_or("?")
            );
        }
    }

    pub fn confirm_dialog_yes(&mut self) {
        let Some(dialog) = self.confirm_dialog.take() else {
            return;
        };
        match dialog.action {
            ConfirmAction::DeletePad => self.confirm_delete_pad(),
            ConfirmAction::ReplaceSound {
                path,
                env_start,
                env_stop,
                duration,
            } => {
                self.start_pad_replace_with_env(&path, env_start, env_stop, duration);
            }
            ConfirmAction::CreatePad => self.create_new_pad(),
            ConfirmAction::MovePad { dst_idx } => self.start_move_pad_transfer(dst_idx),
        }
    }

    pub fn confirm_dialog_no(&mut self) {
        self.confirm_dialog = None;
    }

    pub(super) fn request_full_state(&self) {
        if let Err(e) = self.conn.request_full_state() {
            warn!("failed to request full state: {e}");
        }
    }

    pub(super) fn push_log(&mut self, msg: &str) {
        let t = self.log_start.elapsed().as_secs_f64();
        let entry = format!("[{t:>8.3}] {msg}");
        if self.event_log.len() >= MAX_LOG_ENTRIES {
            self.event_log.pop_front();
        }
        self.event_log.push_back(entry);
        self.log_total += 1;
        if self.log_scroll > 0 {
            self.log_scroll += 1;
        }
        let max = self.event_log.len().saturating_sub(1);
        self.log_scroll = self.log_scroll.min(max);
    }

    fn update_rec_timer(&mut self, prev: RecordingStatus) {
        self.rec_timer.update(prev, self.vm.recorder.state);
    }

    pub fn recording_seconds(&self) -> u64 {
        self.rec_timer.seconds()
    }

    pub fn current_bank(&self) -> BankView {
        self.vm.current_bank_view()
    }

    pub fn selected_pad_info(&self) -> Option<&PadInfo> {
        let logical = self.logical_pad_position();
        let bank = self.vm.selected_bank;
        let ppb = self.profile.pads_per_bank;
        self.vm
            .pads
            .iter()
            .find(|p| p.bank(ppb) == bank && p.position_in_bank(ppb) == logical)
    }

    pub(super) fn logical_pad_position(&self) -> usize {
        BankView::logical_index(self.selected_pad, self.profile)
    }

    pub fn toggle_main_view(&mut self) {
        self.detail_form = None;
        self.main_view = match self.main_view {
            MainView::Pads => MainView::Monitor,
            MainView::Monitor | MainView::Transfer | MainView::Settings => MainView::Pads,
        };
    }

    pub fn enter_settings_view(&mut self) {
        self.detail_form = None;
        self.settings_selected = 0;
        self.settings_item = 0;
        self.settings_focus = SettingsFocus::Categories;
        self.settings_scroll = 0;
        self.settings_draft = SettingsDraft::default();
        self.settings_bt_disconnect = BtDisconnect::Idle;
        self.settings_serial = SerialVisibility::Hidden;
        self.main_view = MainView::Settings;
    }

    pub fn settings_items_focused(&self) -> bool {
        self.settings_focus == SettingsFocus::Items
    }

    fn select_category(&mut self, next: usize) {
        self.settings_selected = next;
        self.settings_item = 0;
        self.settings_scroll = 0;
    }

    pub fn settings_up(&mut self) {
        if self.settings_focus == SettingsFocus::Items {
            self.settings_item = self.settings_item.saturating_sub(1);
            return;
        }
        let count = crate::ui::SETTINGS_CATEGORY_COUNT;
        self.select_category((self.settings_selected + count - 1) % count);
    }

    pub fn settings_down(&mut self) {
        if self.settings_focus == SettingsFocus::Items {
            let last =
                crate::ui::settings_item_count(&self.vm, self.settings_selected).saturating_sub(1);
            self.settings_item = (self.settings_item + 1).min(last);
            return;
        }
        let count = crate::ui::SETTINGS_CATEGORY_COUNT;
        self.select_category((self.settings_selected + 1) % count);
    }

    pub fn settings_enter(&mut self) {
        if self.settings_focus == SettingsFocus::Items {
            self.settings_activate(crate::ui::SettingsStep::Activate);
            return;
        }
        if crate::ui::settings_item_count(&self.vm, self.settings_selected) > 0 {
            self.settings_focus = SettingsFocus::Items;
            self.settings_item = 0;
        }
    }

    pub fn settings_left(&mut self) {
        if self.settings_focus == SettingsFocus::Items {
            self.settings_activate(crate::ui::SettingsStep::Prev);
        }
    }

    pub fn settings_right(&mut self) {
        if self.settings_focus == SettingsFocus::Items {
            self.settings_activate(crate::ui::SettingsStep::Next);
        }
    }

    pub fn settings_back(&mut self) {
        if self.settings_focus == SettingsFocus::Items {
            self.settings_focus = SettingsFocus::Categories;
        } else {
            self.toggle_main_view();
        }
    }

    fn settings_activate(&mut self, step: crate::ui::SettingsStep) {
        match crate::ui::settings_field_role(&self.vm, self.settings_selected, self.settings_item) {
            Some(crate::ui::FieldRole::Live) => self.settings_commit_live(step),
            Some(crate::ui::FieldRole::Stage(field)) => self.settings_stage(field, step),
            Some(crate::ui::FieldRole::Apply) => {
                if matches!(step, crate::ui::SettingsStep::Activate) {
                    self.settings_apply_draft();
                }
            }
            Some(crate::ui::FieldRole::BtDisconnect) => {
                if matches!(step, crate::ui::SettingsStep::Activate) {
                    self.settings_bt_disconnect();
                }
            }
            Some(crate::ui::FieldRole::ToggleSerial) => {
                if matches!(step, crate::ui::SettingsStep::Activate) {
                    self.settings_serial = match self.settings_serial {
                        SerialVisibility::Hidden => SerialVisibility::Shown,
                        SerialVisibility::Shown => SerialVisibility::Hidden,
                    };
                }
            }
            Some(crate::ui::FieldRole::CheckUpdate) => {
                if matches!(step, crate::ui::SettingsStep::Activate) {
                    self.settings_check_update();
                }
            }
            None => {}
        }
    }

    fn settings_check_update(&mut self) {
        if self.vm.system.update.checking {
            return;
        }
        match rcp2_core::ops::system::set_bool(&self.conn, "updateCheckRequested", true) {
            Ok(()) => {
                self.refresh_from_state();
                self.status = "checking for updates\u{2026}".into();
            }
            Err(e) => self.status = format!("update failed: {e}"),
        }
    }

    pub(crate) fn serial_revealed(&self) -> bool {
        self.settings_serial == SerialVisibility::Shown
    }

    fn settings_bt_disconnect(&mut self) {
        if self.settings_bt_disconnect == BtDisconnect::Pending {
            return;
        }
        let address = self.vm.network.bluetooth.connected.clone();
        if address.is_empty() {
            return;
        }
        match rcp2_core::ops::network::disconnect_bluetooth(&self.conn, &address) {
            Ok(()) => {
                self.settings_bt_disconnect = BtDisconnect::Pending;
                self.status = "disconnecting bluetooth\u{2026}".into();
            }
            Err(e) => self.status = format!("update failed: {e}"),
        }
    }

    pub(crate) fn bt_disconnect_pending(&self) -> bool {
        self.settings_bt_disconnect == BtDisconnect::Pending
    }

    fn settings_commit_live(&mut self, step: crate::ui::SettingsStep) {
        let Some((node, name, value)) = crate::ui::settings_live_toggle(
            &self.vm,
            self.settings_selected,
            self.settings_item,
            step,
        ) else {
            return;
        };
        let result = match node {
            crate::ui::SettingsNode::System => {
                rcp2_core::ops::system::set_bool(&self.conn, name, value)
            }
            crate::ui::SettingsNode::Network => {
                rcp2_core::ops::network::set_bool(&self.conn, name, value)
            }
        };
        match result {
            Ok(()) => self.refresh_from_state(),
            Err(e) => self.status = format!("update failed: {e}"),
        }
    }

    fn settings_stage(&mut self, field: crate::ui::DraftField, step: crate::ui::SettingsStep) {
        use crate::ui::DraftField;
        match field {
            DraftField::Lang => {
                self.settings_draft.lang =
                    Some(crate::ui::cycle_language(self.resolved_lang_index(), step));
            }
            DraftField::Timezone => {
                self.settings_draft.timezone =
                    Some(crate::ui::cycle_timezone(self.resolved_tz_index(), step));
            }
        }
    }

    fn settings_apply_draft(&mut self) {
        let fields = crate::ui::category_draft_fields(self.settings_selected);
        let mut changed = false;
        for &field in fields {
            match self.commit_draft_field(field) {
                Ok(true) => changed = true,
                Ok(false) => {}
                Err(e) => {
                    self.status = format!("update failed: {e}");
                    return;
                }
            }
        }
        self.clear_category_draft();
        if changed {
            self.refresh_from_state();
            self.status = if crate::ui::settings_category_is_language(self.settings_selected) {
                "language change sent \u{2014} this may take a moment".into()
            } else {
                "settings applied".into()
            };
        }
    }

    fn commit_draft_field(&self, field: crate::ui::DraftField) -> rcp2_protocol::Result<bool> {
        use crate::ui::DraftField;
        match field {
            DraftField::Timezone => {
                let Some(index) = self.settings_draft.timezone else {
                    return Ok(false);
                };
                if index == self.vm.system.clock.timezone_index as usize {
                    return Ok(false);
                }
                rcp2_core::ops::system::set_u32(
                    &self.conn,
                    "systemDateTimezone",
                    u32::try_from(index).unwrap_or(0),
                )?;
                Ok(true)
            }
            DraftField::Lang => {
                let Some(index) = self.settings_draft.lang else {
                    return Ok(false);
                };
                let code = crate::ui::language_code(index);
                if code == self.vm.system.language {
                    return Ok(false);
                }
                rcp2_core::ops::gui::set_string(&self.conn, "lang", code)?;
                Ok(true)
            }
        }
    }

    fn clear_category_draft(&mut self) {
        use crate::ui::DraftField;
        for &field in crate::ui::category_draft_fields(self.settings_selected) {
            match field {
                DraftField::Lang => self.settings_draft.lang = None,
                DraftField::Timezone => self.settings_draft.timezone = None,
            }
        }
    }

    fn refresh_from_state(&mut self) {
        if let Ok(state) = self.conn.state().snapshot() {
            self.vm.refresh(&state);
        }
    }

    pub(crate) fn resolved_lang_index(&self) -> usize {
        self.settings_draft
            .lang
            .unwrap_or_else(|| crate::ui::language_index(&self.vm.system.language))
    }

    pub(crate) fn resolved_tz_index(&self) -> usize {
        self.settings_draft
            .timezone
            .unwrap_or(self.vm.system.clock.timezone_index as usize)
    }

    pub(crate) fn settings_category_dirty(&self) -> bool {
        crate::ui::category_draft_fields(self.settings_selected)
            .iter()
            .any(|&field| self.draft_field_dirty(field))
    }

    fn draft_field_dirty(&self, field: crate::ui::DraftField) -> bool {
        use crate::ui::DraftField;
        match field {
            DraftField::Lang => self
                .settings_draft
                .lang
                .is_some_and(|i| crate::ui::language_code(i) != self.vm.system.language),
            DraftField::Timezone => self
                .settings_draft
                .timezone
                .is_some_and(|i| i != self.vm.system.clock.timezone_index as usize),
        }
    }

    pub fn settings_scroll_up(&mut self) {
        self.settings_scroll = self.settings_scroll.saturating_sub(1);
    }

    pub fn settings_scroll_down(&mut self) {
        self.settings_scroll = self.settings_scroll.saturating_add(1);
    }

    pub(super) fn require_transfer_tools(&mut self) -> bool {
        if self.has_transfer_tools {
            return true;
        }
        self.status = "transfer requires lsblk and udisksctl".into();
        false
    }

    pub(super) fn require_no_active_download(&mut self) -> bool {
        if self.pad_download.is_none() {
            return true;
        }
        self.status = "download in progress".into();
        false
    }

    pub fn scroll_log_up(&mut self) {
        let max = self.event_log.len().saturating_sub(1);
        self.log_scroll = (self.log_scroll + 1).min(max);
    }

    pub fn scroll_log_down(&mut self) {
        self.log_scroll = self.log_scroll.saturating_sub(1);
    }

    pub fn save_log(&mut self) -> Result<String, std::io::Error> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let path = format!("monitor-{timestamp}.log");
        let mut file = std::fs::File::create(&path)?;
        for entry in &self.event_log {
            writeln!(file, "{entry}")?;
        }
        let count = self.event_log.len();
        self.status = format!("saved {count} entries to {path}");
        Ok(path)
    }

    pub fn next_bank(&mut self) {
        let count = self.vm.bank_count();
        if count > 0 {
            self.vm.selected_bank = (self.vm.selected_bank + 1) % count;
            self.selected_pad = 0;
            self.detail_scroll = 0;
            self.sync_bank_to_device();
        }
    }

    pub fn prev_bank(&mut self) {
        let count = self.vm.bank_count();
        if count > 0 {
            self.vm.selected_bank = (self.vm.selected_bank + count - 1) % count;
            self.selected_pad = 0;
            self.detail_scroll = 0;
            self.sync_bank_to_device();
        }
    }

    pub fn detail_scroll_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(1);
    }

    pub fn detail_scroll_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(1);
    }

    pub fn toggle_recording(&mut self) {
        use rcp2_core::RecordingStatus;
        use rcp2_core::ops::recorder;

        let result = match self.vm.recorder.state {
            RecordingStatus::Recording => recorder::pause_recording(&self.conn),
            RecordingStatus::Stopped | RecordingStatus::Paused => {
                recorder::start_recording(&self.conn)
            }
        };
        if let Err(e) = result {
            self.status = format!("recording failed: {e}");
        }
    }

    pub fn stop_recording(&mut self) {
        use rcp2_core::ops::recorder;

        if let Err(e) = recorder::stop_recording(&self.conn) {
            self.status = format!("stop recording failed: {e}");
        }
    }

    fn sync_bank_to_device(&self) {
        if let Err(e) = rcp2_core::ops::pad::sync_bank(&self.conn, self.vm.selected_bank) {
            warn!("failed to sync bank to device: {e}");
        }
    }

    pub fn next_pad(&mut self) {
        self.selected_pad = (self.selected_pad + 1) % self.profile.pads_per_bank;
        self.detail_scroll = 0;
    }

    pub fn prev_pad(&mut self) {
        let ppb = self.profile.pads_per_bank;
        self.selected_pad = (self.selected_pad + ppb - 1) % ppb;
        self.detail_scroll = 0;
    }

    pub fn select_pad(&mut self, idx: usize) {
        if idx < self.profile.pads_per_bank {
            self.selected_pad = idx;
            self.detail_scroll = 0;
        }
    }
}
