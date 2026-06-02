use log::{info, warn};
use rcp2_protocol::device::DeviceConnection;
use std::time::{Duration, Instant};

use super::pad as pad_ops;
use super::transfer::{PadMove, PadMoveState, TransferState, TransferStatus, host_pad_dir};
use crate::DeviceViewModel;

pub enum PadMoveStatus {
    InProgress(String),
    Done(String),
    Error(String),
}

impl PadMove {
    /// Advances the move state machine by one step.
    ///
    /// # Errors
    /// Returns `PadMoveStatus::Error` if a device operation fails critically.
    pub fn poll(
        &mut self,
        conn: &DeviceConnection,
        transfer: &mut TransferState,
        vm: &DeviceViewModel,
    ) -> PadMoveStatus {
        match self.state {
            PadMoveState::Activating | PadMoveState::WaitingForMount => self.poll_mount(transfer),
            PadMoveState::Moving => self.poll_move(conn, transfer),
            PadMoveState::Deactivating => self.poll_deactivating(conn),
            PadMoveState::Remounting => self.poll_remounting(),
            PadMoveState::CreatingNode => self.poll_create_node(conn, vm),
            PadMoveState::DeletingOld => self.poll_delete_old(conn, vm),
            PadMoveState::Finalizing => self.poll_finalizing(),
            PadMoveState::Done => PadMoveStatus::Done(self.message.clone()),
        }
    }

    fn poll_mount(&mut self, transfer: &mut TransferState) -> PadMoveStatus {
        if self.filename.is_none() {
            self.state = PadMoveState::CreatingNode;
            return PadMoveStatus::InProgress("creating pad node...".into());
        }
        if transfer.find_mount_point() {
            transfer.status = TransferStatus::Active;
            self.state = PadMoveState::Moving;
            PadMoveStatus::InProgress("moving sound file...".into())
        } else {
            self.state = PadMoveState::WaitingForMount;
            PadMoveStatus::InProgress("waiting for mount...".into())
        }
    }

    fn poll_move(
        &mut self,
        conn: &DeviceConnection,
        transfer: &mut TransferState,
    ) -> PadMoveStatus {
        let mount = transfer.mount_point.as_deref().unwrap_or("").to_string();
        let src_dir = host_pad_dir(&mount, self.src_idx);
        let dst_dir = host_pad_dir(&mount, self.dst_idx);

        info!("move: {} -> {}", src_dir.display(), dst_dir.display());

        if dst_dir.exists()
            && let Err(e) = std::fs::remove_dir_all(&dst_dir)
        {
            warn!("failed to remove stale target dir: {e}");
        }

        let msg = match std::fs::rename(&src_dir, &dst_dir) {
            Ok(()) => "sound file moved".to_string(),
            Err(e) => format!("move failed: {e}"),
        };

        self.message.clone_from(&msg);
        self.state = PadMoveState::Deactivating;
        self.state_entered_at = Some(Instant::now());
        Self::finish_transfer(conn, transfer);
        PadMoveStatus::InProgress(msg)
    }

    fn poll_deactivating(&mut self, conn: &DeviceConnection) -> PadMoveStatus {
        let elapsed = self
            .state_entered_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        if elapsed < Duration::from_millis(500) {
            return PadMoveStatus::InProgress("waiting before remount...".into());
        }
        if let Err(e) = pad_ops::remount_pad_storage(conn) {
            warn!("failed to remount pad storage: {e}");
            return PadMoveStatus::Error(format!("failed to remount pad storage: {e}"));
        }
        self.state = PadMoveState::Remounting;
        self.state_entered_at = Some(Instant::now());
        PadMoveStatus::InProgress("remounting pad storage...".into())
    }

    fn poll_remounting(&mut self) -> PadMoveStatus {
        let elapsed = self
            .state_entered_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        if elapsed < Duration::from_secs(1) {
            return PadMoveStatus::InProgress("remounting pad storage...".into());
        }
        self.state = PadMoveState::CreatingNode;
        PadMoveStatus::InProgress("creating pad node...".into())
    }

    fn poll_create_node(&mut self, conn: &DeviceConnection, vm: &DeviceViewModel) -> PadMoveStatus {
        let Some(sp_idx) = vm.soundpads_idx else {
            self.state = PadMoveState::Done;
            return PadMoveStatus::Error("SOUNDPADS not found".into());
        };

        let device_path = match &self.filename {
            Some(name) => pad_ops::pad_dir_file_path(self.dst_idx, name),
            None => String::new(),
        };

        let insert_at = vm.pads.len();
        match pad_ops::create_pad_node_from_props(
            conn,
            sp_idx,
            insert_at,
            self.dst_idx,
            &device_path,
            &self.props,
        ) {
            Ok(()) => {
                self.state = PadMoveState::DeletingOld;
                PadMoveStatus::InProgress("removing old pad...".into())
            }
            Err(e) => {
                self.state = PadMoveState::Done;
                PadMoveStatus::Error(format!("failed to create pad node: {e}"))
            }
        }
    }

    fn poll_delete_old(&mut self, conn: &DeviceConnection, vm: &DeviceViewModel) -> PadMoveStatus {
        let Some(sp_idx) = vm.soundpads_idx else {
            self.state = PadMoveState::Done;
            return PadMoveStatus::Error("SOUNDPADS not found".into());
        };

        match pad_ops::delete_pad(conn, sp_idx, self.src_child_index) {
            Ok(()) => {
                if let Err(e) = pad_ops::remount_pad_storage(conn) {
                    warn!("failed to remount pad storage: {e}");
                }
                self.message = format!("moved pad: {}", self.pad_name);
                self.state = PadMoveState::Finalizing;
                self.state_entered_at = Some(Instant::now());
                PadMoveStatus::InProgress("reloading pad storage...".into())
            }
            Err(e) => {
                self.state = PadMoveState::Done;
                PadMoveStatus::Error(format!("failed to remove old pad: {e}"))
            }
        }
    }

    fn poll_finalizing(&mut self) -> PadMoveStatus {
        let elapsed = self
            .state_entered_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        if elapsed < Duration::from_secs(1) {
            return PadMoveStatus::InProgress("reloading pad storage...".into());
        }
        self.state = PadMoveState::Done;
        PadMoveStatus::Done(self.message.clone())
    }

    fn finish_transfer(conn: &DeviceConnection, transfer: &mut TransferState) {
        transfer.unmount();
        if let Err(e) = pad_ops::deactivate_transfer_mode(conn) {
            warn!("failed to deactivate transfer mode: {e}");
        }
        transfer.status = TransferStatus::Inactive;
    }
}
