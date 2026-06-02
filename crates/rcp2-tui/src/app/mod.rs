mod pad_ops;
mod transfer_ops;

use crate::detail_form::DetailForm;
use crate::transfer::{PadDownload, PadUpload, TransferState, TransferStatus};
use log::{info, warn};
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
    pub confirm_dialog: Option<ConfirmDialog>,
    pub(super) rec_started_at: Option<Instant>,
    pub(super) rec_paused_elapsed: u64,
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
            confirm_dialog: None,
            rec_started_at: None,
            rec_paused_elapsed: 0,
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
                        let button_pos =
                            indices[1].saturating_sub(self.profile.padbutton_offset);
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
                    self.rec_started_at = None;
                    self.rec_paused_elapsed = 0;
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
        }

        self.poll_pad_download();
        self.poll_pad_upload();

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
        let cur = self.vm.recorder.state;
        if prev == cur {
            return;
        }
        match cur {
            RecordingStatus::Recording => {
                self.rec_started_at = Some(Instant::now());
            }
            RecordingStatus::Paused => {
                if let Some(started) = self.rec_started_at {
                    self.rec_paused_elapsed += started.elapsed().as_secs();
                }
                self.rec_started_at = None;
            }
            RecordingStatus::Stopped => {
                self.rec_started_at = None;
                self.rec_paused_elapsed = 0;
            }
        }
    }

    pub fn recording_seconds(&self) -> u64 {
        let active = self.rec_started_at.map_or(0, |t| t.elapsed().as_secs());
        self.rec_paused_elapsed + active
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
            MainView::Monitor | MainView::Transfer => MainView::Pads,
        };
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
            self.sync_bank_to_device();
        }
    }

    pub fn prev_bank(&mut self) {
        let count = self.vm.bank_count();
        if count > 0 {
            self.vm.selected_bank = (self.vm.selected_bank + count - 1) % count;
            self.selected_pad = 0;
            self.sync_bank_to_device();
        }
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
    }

    pub fn prev_pad(&mut self) {
        let ppb = self.profile.pads_per_bank;
        self.selected_pad = (self.selected_pad + ppb - 1) % ppb;
    }

    pub fn select_pad(&mut self, idx: usize) {
        if idx < self.profile.pads_per_bank {
            self.selected_pad = idx;
        }
    }
}
