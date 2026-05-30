use crate::transport::Transport;
use std::collections::VecDeque;

pub struct MockTransport {
    rx_queue: VecDeque<Vec<u8>>,
    tx_log: Vec<Vec<u8>>,
}

impl MockTransport {
    #[must_use]
    pub fn new(rx_frames: Vec<Vec<u8>>) -> Self {
        MockTransport {
            rx_queue: rx_frames.into(),
            tx_log: Vec::new(),
        }
    }

    #[must_use]
    pub fn sent_frames(&self) -> &[Vec<u8>] {
        &self.tx_log
    }
}

impl Transport for MockTransport {
    fn send(&mut self, data: &[u8]) -> crate::Result<()> {
        self.tx_log.push(data.to_vec());
        Ok(())
    }

    fn recv(&mut self) -> crate::Result<Vec<u8>> {
        self.rx_queue
            .pop_front()
            .ok_or_else(|| crate::Error::Transport("mock: no more frames".into()))
    }
}
