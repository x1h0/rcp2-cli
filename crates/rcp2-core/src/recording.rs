use crate::RecordingStatus;
use std::time::Instant;

// The device only reports a coarse recordTimeMs, so elapsed seconds are tracked
// locally across record/pause/stop transitions.
#[derive(Debug, Default)]
pub struct RecordingTimer {
    started_at: Option<Instant>,
    paused_elapsed: u64,
}

impl RecordingTimer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, prev: RecordingStatus, cur: RecordingStatus) {
        if prev == cur {
            return;
        }
        match cur {
            RecordingStatus::Recording => {
                self.started_at = Some(Instant::now());
            }
            RecordingStatus::Paused => {
                if let Some(started) = self.started_at {
                    self.paused_elapsed += started.elapsed().as_secs();
                }
                self.started_at = None;
            }
            RecordingStatus::Stopped => self.reset(),
        }
    }

    pub fn reset(&mut self) {
        self.started_at = None;
        self.paused_elapsed = 0;
    }

    #[must_use]
    pub fn seconds(&self) -> u64 {
        let active = self.started_at.map_or(0, |t| t.elapsed().as_secs());
        self.paused_elapsed + active
    }
}
