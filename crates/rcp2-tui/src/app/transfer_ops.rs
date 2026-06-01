use super::App;
use crate::transfer::{PadDownload, PadDownloadState, PadDownloadStatus, PadUploadState};
use rcp2_core::ops::TRANSFER_MODE_SD;
use rcp2_core::ops::pad as pad_ops;

impl App {
    pub fn enter_transfer_view(&mut self) {
        if self.dry_run {
            self.status = "transfer disabled in dry-run".into();
            return;
        }
        if !self.require_transfer_tools() {
            return;
        }
        if self.transfer.status == crate::transfer::TransferStatus::Active {
            self.main_view = super::MainView::Transfer;
            return;
        }
        self.main_view = super::MainView::Transfer;
        self.transfer.storage_choice = None;
    }

    pub fn choose_transfer_storage(&mut self, mode: u32) {
        if mode == TRANSFER_MODE_SD && !self.vm.has_storage() {
            self.status = "no SD card detected".into();
            return;
        }
        self.transfer.storage_choice = Some(mode);
        self.activate_transfer_mode(mode);
    }

    pub fn leave_transfer_view(&mut self) {
        if self.transfer.status == crate::transfer::TransferStatus::Active
            || self.transfer.status == crate::transfer::TransferStatus::Activating
        {
            self.deactivate_transfer_mode();
        }
        self.main_view = super::MainView::Pads;
    }

    pub(super) fn activate_transfer_mode(&mut self, mode: u32) {
        if !self.require_transfer_tools() {
            self.transfer.status = crate::transfer::TransferStatus::Error;
            return;
        }
        if let Err(e) = pad_ops::activate_transfer_mode(&self.conn, mode) {
            self.status = format!("transfer mode failed: {e}");
            self.transfer.status = crate::transfer::TransferStatus::Error;
            return;
        }
        self.transfer.status = crate::transfer::TransferStatus::Activating;
        self.status = "activating transfer mode...".into();
    }

    pub(super) fn deactivate_transfer_mode(&mut self) {
        self.transfer.unmount();
        if let Err(e) = pad_ops::deactivate_transfer_mode(&self.conn) {
            self.status = format!("failed to deactivate transfer mode: {e}");
            return;
        }
        self.transfer.status = crate::transfer::TransferStatus::Inactive;
        self.status = "transfer mode deactivated".into();
    }

    pub fn transfer_enter(&mut self) {
        if let Some(ref prompt) = self.transfer.save_prompt {
            if prompt.source.is_dir() {
                self.transfer.confirm_dir_download();
            } else {
                self.transfer.confirm_download();
            }
            return;
        }
        let Some(entry) = self.transfer.files.get(self.transfer.selected) else {
            return;
        };
        if entry.is_dir {
            self.transfer.enter_dir();
        } else {
            self.transfer.start_download();
        }
    }

    pub fn transfer_download_selected(&mut self) {
        let Some(entry) = self.transfer.files.get(self.transfer.selected) else {
            return;
        };
        if entry.is_dir && entry.name != ".." {
            self.transfer.start_dir_download();
        } else if !entry.is_dir {
            self.transfer.start_download();
        }
    }

    pub fn transfer_cancel(&mut self) {
        if self.transfer.save_prompt.is_some() {
            self.transfer.cancel_download();
        } else {
            self.leave_transfer_view();
        }
    }

    pub fn transfer_input(&mut self, c: char) {
        if let Some(ref mut prompt) = self.transfer.save_prompt {
            prompt.input(c);
        }
    }

    pub fn transfer_backspace(&mut self) {
        if let Some(ref mut prompt) = self.transfer.save_prompt {
            prompt.backspace();
        }
    }

    pub fn has_save_prompt(&self) -> bool {
        self.transfer.save_prompt.is_some()
    }

    pub fn transfer_select_up(&mut self) {
        if !self.transfer.files.is_empty() {
            self.transfer.selected = self.transfer.selected.saturating_sub(1);
        }
    }

    pub fn transfer_select_down(&mut self) {
        if !self.transfer.files.is_empty() {
            self.transfer.selected =
                (self.transfer.selected + 1).min(self.transfer.files.len() - 1);
        }
    }

    pub fn start_pad_download(&mut self) {
        if self.dry_run {
            self.status = "download disabled in dry-run".into();
            return;
        }
        if !self.require_transfer_tools() {
            return;
        }
        if self.pad_upload.is_some() {
            self.status = "upload in progress".into();
            return;
        }
        let Some(pad) = self.selected_pad_info() else {
            return;
        };
        if pad.file_path.is_empty() {
            self.status = "no sound file on this pad".into();
            return;
        }
        self.pad_download = Some(PadDownload::new(&pad.file_path, &pad.name));
    }

    pub fn has_pad_download_prompt(&self) -> bool {
        self.pad_download
            .as_ref()
            .is_some_and(|d| d.state == PadDownloadState::Prompting)
    }

    pub fn confirm_pad_download(&mut self) {
        if let Some(ref mut dl) = self.pad_download {
            dl.save_path = dl.prompt.input.clone();
            dl.state = PadDownloadState::Activating;
            self.status = "activating transfer mode...".into();
            self.activate_transfer_mode(2);
        }
    }

    pub fn cancel_pad_download(&mut self) {
        if let Some(ref dl) = self.pad_download
            && dl.state != PadDownloadState::Prompting
        {
            self.deactivate_transfer_mode();
        }
        self.pad_download = None;
    }

    pub fn pad_download_input(&mut self, c: char) {
        if let Some(ref mut dl) = self.pad_download {
            dl.prompt.input(c);
        }
    }

    pub fn pad_download_backspace(&mut self) {
        if let Some(ref mut dl) = self.pad_download {
            dl.prompt.backspace();
        }
    }

    pub fn has_pad_upload_prompt(&self) -> bool {
        self.pad_upload
            .as_ref()
            .is_some_and(|u| u.state == PadUploadState::Prompting)
    }

    pub fn confirm_pad_upload(&mut self) {
        if let Some(ref mut ul) = self.pad_upload {
            let path = ul.prompt.input.clone();
            if !std::path::Path::new(&path).exists() {
                self.status = format!("file not found: {path}");
                return;
            }
            ul.source_path = path;
            ul.state = PadUploadState::Activating;
            self.status = "activating transfer mode for upload...".into();
            self.activate_transfer_mode(2);
        }
    }

    pub fn cancel_pad_upload(&mut self) {
        if let Some(ref ul) = self.pad_upload
            && ul.state != PadUploadState::Prompting
        {
            self.deactivate_transfer_mode();
        }
        self.pad_upload = None;
    }

    pub fn pad_upload_input(&mut self, c: char) {
        if let Some(ref mut ul) = self.pad_upload {
            ul.prompt.input(c);
        }
    }

    pub fn pad_upload_backspace(&mut self) {
        if let Some(ref mut ul) = self.pad_upload {
            ul.prompt.backspace();
        }
    }

    pub(super) fn poll_pad_download(&mut self) {
        let Some(ref mut dl) = self.pad_download else {
            return;
        };
        match dl.poll(&self.conn, &mut self.transfer) {
            PadDownloadStatus::InProgress(msg) => self.status = msg,
            PadDownloadStatus::Done(msg) | PadDownloadStatus::Error(msg) => {
                self.status = msg;
                self.pad_download = None;
            }
        }
    }
}
