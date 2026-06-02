use log::{info, warn};
use rcp2_protocol::device::DeviceConnection;
use rcp2_protocol::types::Value;
use std::time::{Duration, Instant};

use super::format_size;
use super::pad as pad_ops;
use super::transfer::{TransferState, TransferStatus};
use crate::DeviceViewModel;

pub use super::transfer::{PadUpload, PadUploadState};

pub enum PadUploadStatus {
    InProgress(String),
    Done(String),
    Error(String),
}

impl PadUpload {
    #[must_use]
    pub fn start_new(
        pad_idx: usize,
        source_path: String,
        pad_name: String,
        env_start: Option<f64>,
        env_stop: Option<f64>,
    ) -> Self {
        let mut ul = Self::new(pad_idx);
        ul.source_path = source_path;
        ul.state = PadUploadState::Activating;
        ul.is_new_pad = true;
        ul.pad_name = pad_name;
        ul.env_start = env_start;
        ul.env_stop = env_stop;
        ul
    }

    #[must_use]
    pub fn prepare_replace(
        conn: &DeviceConnection,
        soundpads_idx: usize,
        child_index: usize,
        pad_idx: usize,
        source_path: &str,
        ext: &str,
        env: (Option<f64>, Option<f64>),
    ) -> Self {
        let (env_start, env_stop) = env;
        let new_file_path = pad_ops::device_pad_path(pad_idx, ext);

        if let Err(e) = pad_ops::send_property(
            conn,
            soundpads_idx,
            child_index,
            "padFilePath",
            Value::String(new_file_path),
        ) {
            warn!("failed to update padFilePath: {e}");
        }

        let start = env_start.unwrap_or(0.0).clamp(0.0, 1.0);
        let stop = env_stop.unwrap_or(1.0).clamp(0.0, 1.0);

        if let Err(e) = pad_ops::send_property(
            conn,
            soundpads_idx,
            child_index,
            "padEnvStart",
            Value::F64(start),
        ) {
            warn!("failed to update padEnvStart: {e}");
        }
        if let Err(e) = pad_ops::send_property(
            conn,
            soundpads_idx,
            child_index,
            "padEnvStop",
            Value::F64(stop),
        ) {
            warn!("failed to update padEnvStop: {e}");
        }

        let mut ul = Self::new(pad_idx);
        ul.source_path = source_path.to_string();
        ul.state = PadUploadState::Activating;
        ul
    }

    /// Advances the upload state machine by one step.
    ///
    /// # Errors
    /// Returns `PadUploadStatus::Error` if a device operation fails critically.
    pub fn poll(
        &mut self,
        conn: &DeviceConnection,
        transfer: &mut TransferState,
        vm: &DeviceViewModel,
    ) -> PadUploadStatus {
        match self.state {
            PadUploadState::Activating | PadUploadState::WaitingForMount => {
                self.poll_mount(transfer)
            }
            PadUploadState::Copying => self.poll_copy(conn, transfer),
            PadUploadState::Deactivating => self.poll_deactivating(conn),
            PadUploadState::Remounting => self.poll_remounting(),
            PadUploadState::CreatingNode => self.poll_create_node(conn, vm),
            PadUploadState::Finalizing => self.poll_finalizing(),
            PadUploadState::Done => PadUploadStatus::Done(self.message.clone()),
            PadUploadState::Prompting => PadUploadStatus::InProgress(String::new()),
        }
    }

    fn poll_mount(&mut self, transfer: &mut TransferState) -> PadUploadStatus {
        if transfer.find_mount_point() {
            transfer.status = TransferStatus::Active;
            self.state = PadUploadState::Copying;
            PadUploadStatus::InProgress("uploading file...".into())
        } else {
            self.state = PadUploadState::WaitingForMount;
            PadUploadStatus::InProgress("waiting for mount...".into())
        }
    }

    fn poll_copy(
        &mut self,
        conn: &DeviceConnection,
        transfer: &mut TransferState,
    ) -> PadUploadStatus {
        let mount = transfer.mount_point.as_deref().unwrap_or("").to_string();
        let target_dir = self.host_target_dir(&mount);
        let source = std::path::Path::new(&self.source_path);
        let ext = source
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if ext != "wav" && ext != "mp3" {
            self.message = "only WAV and MP3 files are supported".into();
            self.state = PadUploadState::Deactivating;
            Self::finish_transfer(conn, transfer);
            return PadUploadStatus::Error(self.message.clone());
        }

        let filename = format!("sound.{ext}");
        info!(
            "upload: {} -> {}/{}",
            source.display(),
            target_dir.display(),
            filename
        );

        let msg = if let Err(e) = std::fs::create_dir_all(&target_dir) {
            format!("mkdir failed: {e}")
        } else {
            let dest = target_dir.join(&filename);
            match std::fs::copy(source, &dest) {
                Ok(bytes) => {
                    self.uploaded_filename = Some(filename.clone());
                    format!("uploaded {} ({})", filename, format_size(bytes))
                }
                Err(e) => format!("upload failed: {e}"),
            }
        };

        self.message.clone_from(&msg);
        self.state = PadUploadState::Deactivating;
        self.state_entered_at = Some(Instant::now());
        Self::finish_transfer(conn, transfer);
        PadUploadStatus::InProgress(msg)
    }

    fn poll_deactivating(&mut self, conn: &DeviceConnection) -> PadUploadStatus {
        let elapsed = self
            .state_entered_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        if elapsed < Duration::from_millis(500) {
            return PadUploadStatus::InProgress("waiting before remount...".into());
        }
        if let Err(e) = pad_ops::remount_pad_storage(conn) {
            warn!("failed to remount pad storage: {e}");
            return PadUploadStatus::Error(format!("failed to remount pad storage: {e}"));
        }
        self.state = PadUploadState::Remounting;
        self.state_entered_at = Some(Instant::now());
        PadUploadStatus::InProgress("remounting pad storage...".into())
    }

    fn poll_remounting(&mut self) -> PadUploadStatus {
        let elapsed = self
            .state_entered_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        if elapsed < Duration::from_secs(1) {
            return PadUploadStatus::InProgress("remounting pad storage...".into());
        }
        if self.is_new_pad {
            self.state = PadUploadState::CreatingNode;
            PadUploadStatus::InProgress("creating pad node...".into())
        } else {
            self.state = PadUploadState::Done;
            PadUploadStatus::InProgress(self.message.clone())
        }
    }

    fn poll_create_node(
        &mut self,
        conn: &DeviceConnection,
        vm: &DeviceViewModel,
    ) -> PadUploadStatus {
        let filename = self
            .uploaded_filename
            .clone()
            .unwrap_or_else(|| "sound.wav".into());

        let Some(sp_idx) = vm.soundpads_idx else {
            self.state = PadUploadState::Done;
            return PadUploadStatus::Error("SOUNDPADS not found".into());
        };

        let insert_at = vm.pads.len();
        match pad_ops::create_pad_node(
            conn,
            sp_idx,
            insert_at,
            self.pad_idx,
            &filename,
            &self.pad_name,
        ) {
            Ok(()) => {
                self.send_envelope(conn, sp_idx, insert_at);
                if let Err(e) = pad_ops::remount_pad_storage(conn) {
                    warn!("failed to remount pad storage: {e}");
                }
                self.message = format!("pad created: {}", self.pad_name);
                self.state = PadUploadState::Finalizing;
                self.state_entered_at = Some(Instant::now());
                PadUploadStatus::InProgress("reloading pad storage...".into())
            }
            Err(e) => {
                self.state = PadUploadState::Done;
                PadUploadStatus::Error(format!("failed to create pad node: {e}"))
            }
        }
    }

    fn poll_finalizing(&mut self) -> PadUploadStatus {
        let elapsed = self
            .state_entered_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        if elapsed < Duration::from_secs(1) {
            return PadUploadStatus::InProgress("reloading pad storage...".into());
        }
        self.state = PadUploadState::Done;
        PadUploadStatus::Done(self.message.clone())
    }

    fn send_envelope(&self, conn: &DeviceConnection, sp_idx: usize, child_idx: usize) {
        if let (Some(start), Some(stop)) = (self.env_start, self.env_stop) {
            if let Err(e) = pad_ops::send_property_to_device(
                conn,
                sp_idx,
                child_idx,
                "padEnvStart",
                Value::F64(start.clamp(0.0, 1.0)),
            ) {
                warn!("failed to set padEnvStart: {e}");
            }
            if let Err(e) = pad_ops::send_property_to_device(
                conn,
                sp_idx,
                child_idx,
                "padEnvStop",
                Value::F64(stop.clamp(0.0, 1.0)),
            ) {
                warn!("failed to set padEnvStop: {e}");
            }
        }
    }

    fn finish_transfer(conn: &DeviceConnection, transfer: &mut TransferState) {
        transfer.unmount();
        if let Err(e) = pad_ops::deactivate_transfer_mode(conn) {
            warn!("failed to deactivate transfer mode: {e}");
        }
        transfer.status = TransferStatus::Inactive;
    }
}
