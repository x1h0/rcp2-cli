use log::{info, warn};
use rcp2_protocol::device::DeviceConnection;

use super::format_size;
use super::pad as pad_ops;
use super::transfer::{TransferState, TransferStatus};

pub use super::transfer::{PadDownload, PadDownloadState};

pub enum PadDownloadStatus {
    InProgress(String),
    Done(String),
    Error(String),
}

impl PadDownload {
    pub fn poll(
        &mut self,
        conn: &DeviceConnection,
        transfer: &mut TransferState,
    ) -> PadDownloadStatus {
        match self.state {
            PadDownloadState::Activating | PadDownloadState::WaitingForMount => {
                if transfer.find_mount_point() {
                    transfer.status = TransferStatus::Active;
                    self.state = PadDownloadState::Copying;
                    PadDownloadStatus::InProgress("copying file...".into())
                } else {
                    self.state = PadDownloadState::WaitingForMount;
                    PadDownloadStatus::InProgress("waiting for mount...".into())
                }
            }
            PadDownloadState::Copying => {
                let mount = transfer.mount_point.as_deref().unwrap_or("");
                let src = self.host_file_path(mount);
                let dest = std::path::Path::new(&self.save_path);

                info!("pad download: {} -> {}", src.display(), dest.display());

                let mut msg = if src.exists() {
                    match std::fs::copy(&src, dest) {
                        Ok(bytes) => {
                            format!("saved to {} ({})", dest.display(), format_size(bytes))
                        }
                        Err(e) => format!("copy failed: {e}"),
                    }
                } else {
                    format!("file not found: {}", src.display())
                };

                self.state = PadDownloadState::Deactivating;

                transfer.unmount();
                if let Err(e) = pad_ops::deactivate_transfer_mode(conn) {
                    warn!("failed to deactivate transfer mode: {e}");
                    msg = format!("{msg} (warning: failed to deactivate transfer mode: {e})");
                }
                self.message = msg;
                transfer.status = TransferStatus::Inactive;

                PadDownloadStatus::InProgress(self.message.clone())
            }
            PadDownloadState::Deactivating => {
                self.state = PadDownloadState::Done;
                PadDownloadStatus::InProgress(self.message.clone())
            }
            PadDownloadState::Done => PadDownloadStatus::Done(self.message.clone()),
            PadDownloadState::Prompting => PadDownloadStatus::InProgress(String::new()),
        }
    }
}
