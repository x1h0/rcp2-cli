use crate::device::model::DeviceModel;
use crate::device::state::DeviceState;
use crate::framing::{self, PacketResult};
use crate::packet::PacketSerialize;
use crate::packet::handshake::HANDSHAKE_BYTES;
use crate::packet::property_update::PropertyUpdatePacket;
use crate::transport::Transport;
use crate::transport::hid::FRAME_PAYLOAD_SIZE;
use crate::types::Value;
use log::{debug, error, info, trace};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

#[derive(Debug, Clone)]
pub enum DeviceEvent {
    StateInitialized,
    PropertyUpdated {
        indices: Vec<usize>,
        name: String,
        value: Value,
    },
    UnknownPacket(Vec<u8>),
    Error(String),
    Disconnected,
}

enum TxCommand {
    SendPropertyUpdate {
        indices: Vec<usize>,
        name: String,
        value: Value,
    },
    SendRawPacket(Box<dyn PacketSerialize + Send>),
    ResendHandshake,
    Disconnect,
}

pub struct DeviceConnection {
    model: DeviceModel,
    state: DeviceState,
    event_rx: mpsc::Receiver<DeviceEvent>,
    tx_cmd: mpsc::Sender<TxCommand>,
    shutdown: Arc<AtomicBool>,
    rx_thread: Option<thread::JoinHandle<()>>,
    tx_thread: Option<thread::JoinHandle<()>>,
}

impl DeviceConnection {
    /// Opens a connection to the device using separate RX/TX transports.
    ///
    /// # Errors
    /// Returns an error if the handshake send fails or threads cannot be spawned.
    pub fn open(
        mut rx_transport: Box<dyn Transport>,
        mut tx_transport: Box<dyn Transport>,
        model: DeviceModel,
    ) -> crate::Result<Self> {
        info!("sending handshake");
        tx_transport.send(HANDSHAKE_BYTES)?;
        debug!("handshake sent: {HANDSHAKE_BYTES:02x?}");

        let state = DeviceState::new();
        let state_clone = state.clone();
        let (event_tx, event_rx) = mpsc::channel();
        let (tx_cmd, tx_cmd_rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_rx = shutdown.clone();
        let rx_handle = thread::Builder::new()
            .name("HidSyncRxThread".into())
            .spawn(move || {
                Self::rx_loop(&mut *rx_transport, &state_clone, &event_tx, &shutdown_rx);
                debug!("RX thread stopped");
            })
            .map_err(|e| crate::Error::Transport(format!("failed to spawn RX thread: {e}")))?;

        let shutdown_tx = shutdown.clone();
        let tx_handle = thread::Builder::new()
            .name("HidSyncTxThread".into())
            .spawn(move || {
                Self::tx_loop(&mut *tx_transport, &tx_cmd_rx, &shutdown_tx);
                debug!("TX thread stopped");
            })
            .map_err(|e| crate::Error::Transport(format!("failed to spawn TX thread: {e}")))?;

        Ok(DeviceConnection {
            model,
            state,
            event_rx,
            tx_cmd,
            shutdown,
            rx_thread: Some(rx_handle),
            tx_thread: Some(tx_handle),
        })
    }

    #[must_use]
    pub fn model(&self) -> DeviceModel {
        self.model
    }

    #[must_use]
    pub fn state(&self) -> &DeviceState {
        &self.state
    }

    #[must_use]
    pub fn events(&self) -> &mpsc::Receiver<DeviceEvent> {
        &self.event_rx
    }

    /// Requests the device to resend its full state.
    ///
    /// # Errors
    /// Returns an error if the TX channel is closed.
    pub fn request_full_state(&self) -> crate::Result<()> {
        self.tx_cmd
            .send(TxCommand::ResendHandshake)
            .map_err(|e| crate::Error::Transport(format!("TX channel closed: {e}")))
    }

    /// Sends a raw packet to the device.
    ///
    /// # Errors
    /// Returns an error if the TX channel is closed.
    pub fn send_packet(&self, packet: Box<dyn PacketSerialize + Send>) -> crate::Result<()> {
        self.tx_cmd
            .send(TxCommand::SendRawPacket(packet))
            .map_err(|e| crate::Error::Transport(format!("TX channel closed: {e}")))
    }

    /// Sends a property update to the device.
    ///
    /// # Errors
    /// Returns an error if the TX channel is closed.
    pub fn send_property_update(
        &self,
        indices: Vec<usize>,
        name: String,
        value: Value,
    ) -> crate::Result<()> {
        self.tx_cmd
            .send(TxCommand::SendPropertyUpdate {
                indices,
                name,
                value,
            })
            .map_err(|e| crate::Error::Transport(format!("TX channel closed: {e}")))
    }

    pub fn disconnect(&mut self) {
        if self.shutdown.load(Ordering::SeqCst) {
            return;
        }
        info!("disconnecting...");
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = self.tx_cmd.send(TxCommand::Disconnect);
        if let Some(h) = self.tx_thread.take() {
            let _ = h.join();
        }
        if let Some(h) = self.rx_thread.take() {
            let _ = h.join();
        }
        info!("disconnected");
    }

    /// Blocks until the device sends its full state tree.
    ///
    /// # Errors
    /// Returns an error if the device disconnects or the event channel closes.
    pub fn wait_for_state(&self) -> crate::Result<()> {
        info!("waiting for device state...");
        loop {
            match self.event_rx.recv() {
                Ok(DeviceEvent::StateInitialized) => {
                    info!("device state received");
                    return Ok(());
                }
                Ok(DeviceEvent::Error(e)) => {
                    return Err(crate::Error::Transport(e));
                }
                Ok(DeviceEvent::Disconnected) => {
                    return Err(crate::Error::Transport("device disconnected".into()));
                }
                Ok(_) => {}
                Err(_) => {
                    return Err(crate::Error::Transport("event channel closed".into()));
                }
            }
        }
    }

    fn rx_loop(
        transport: &mut dyn Transport,
        state: &DeviceState,
        events: &mpsc::Sender<DeviceEvent>,
        shutdown: &AtomicBool,
    ) {
        loop {
            if shutdown.load(Ordering::SeqCst) {
                return;
            }

            let recv_with_shutdown = || -> crate::Result<Vec<u8>> {
                loop {
                    if shutdown.load(Ordering::SeqCst) {
                        return Err(crate::Error::Transport("shutdown".into()));
                    }
                    match transport.recv() {
                        Err(crate::Error::Timeout) => {}
                        other => return other,
                    }
                }
            };

            match framing::read_framed_message(recv_with_shutdown) {
                Ok(PacketResult::DeviceReport(report)) => {
                    trace!("received device report: root={}", report.report.name);
                    if let Err(e) = state.replace(report.report) {
                        error!("failed to replace state: {e}");
                        continue;
                    }
                    let _ = events.send(DeviceEvent::StateInitialized);
                }
                Ok(PacketResult::PropertyUpdate(update)) => {
                    if !state.is_initialized().unwrap_or(false) {
                        trace!(
                            "ignoring property update before state init: {}",
                            update.name
                        );
                        continue;
                    }
                    trace!(
                        "property update: {:?} {} = {:?}",
                        update.indices, update.name, update.value
                    );
                    let value = update.value.clone();
                    if let Err(e) = state.set_property(&update.indices, &update.name, update.value)
                    {
                        debug!("skipped property update: {e}");
                    }
                    let _ = events.send(DeviceEvent::PropertyUpdated {
                        indices: update.indices,
                        name: update.name,
                        value,
                    });
                }
                Ok(PacketResult::Unknown(data)) => {
                    debug!("unknown packet: {} bytes", data.len());
                    let _ = events.send(DeviceEvent::UnknownPacket(data));
                }
                Err(e) => {
                    if shutdown.load(Ordering::SeqCst) {
                        return;
                    }
                    error!("read error: {e}");
                    let _ = events.send(DeviceEvent::Error(e.to_string()));
                    let _ = events.send(DeviceEvent::Disconnected);
                    return;
                }
            }
        }
    }

    fn tx_loop(
        transport: &mut dyn Transport,
        commands: &mpsc::Receiver<TxCommand>,
        shutdown: &AtomicBool,
    ) {
        while let Ok(cmd) = commands.recv() {
            if shutdown.load(Ordering::SeqCst) {
                return;
            }

            match cmd {
                TxCommand::Disconnect => {
                    return;
                }
                TxCommand::SendPropertyUpdate {
                    indices,
                    name,
                    value,
                } => {
                    let packet = PropertyUpdatePacket {
                        indices,
                        name,
                        value,
                    };
                    if let Err(e) =
                        framing::write_framed_message(&packet, FRAME_PAYLOAD_SIZE, |frame| {
                            transport.send(frame)
                        })
                    {
                        error!("failed to send property update: {e}");
                    }
                }
                TxCommand::ResendHandshake => {
                    debug!("re-sending handshake for full state refresh");
                    let _ = transport.send(HANDSHAKE_BYTES);
                }
                TxCommand::SendRawPacket(packet) => {
                    if let Err(e) =
                        framing::write_framed_message(&*packet, FRAME_PAYLOAD_SIZE, |frame| {
                            transport.send(frame)
                        })
                    {
                        error!("failed to send packet: {e}");
                    }
                }
            }
        }
    }
}

impl Drop for DeviceConnection {
    fn drop(&mut self) {
        self.disconnect();
    }
}
