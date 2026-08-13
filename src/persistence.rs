//! Settled, coalescing persistence outside the event-loop thread.

use crate::{
    NativeWake,
    responsiveness::{SupersedingReceiver, SupersedingSender, superseding_channel},
};
use anyhow::{Context as _, Result, anyhow, ensure};
use crossbeam_channel::{Sender, bounded};
use std::{
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Latest background persistence result.
#[derive(Debug)]
pub enum ScribeOutcome {
    /// The latest submitted snapshot reached durable storage.
    Saved { sequence: u64 },
    /// The latest submitted snapshot could not be saved.
    Fault { sequence: u64, message: String },
}

struct Inscription<T> {
    sequence: u64,
    snapshot: T,
    receipt: Option<Sender<Result<(), String>>>,
}

/// A settled, latest-snapshot-wins background persistence worker.
///
/// Mark the scribe whenever its source state changes, expose [`Self::deadline`]
/// through `NativeApp::service_deadline`, and call [`Self::tend`] when that
/// deadline matures. Ordinary writes never block the event-loop thread. A
/// failed write is reported but not retried automatically; the next mutation,
/// an explicit submit, or orderly retirement decides when to try again.
pub struct SettledScribe<T> {
    settle: Duration,
    dirty: Option<Instant>,
    sequence: u64,
    inscriptions: Option<SupersedingSender<Inscription<T>>>,
    outcomes: SupersedingReceiver<ScribeOutcome>,
    thread: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> SettledScribe<T> {
    /// Raise a named persistence worker.
    pub fn spawn(
        name: impl Into<String>,
        ctx: &egui::Context,
        settle: Duration,
        mut write: impl FnMut(T) -> Result<()> + Send + 'static,
    ) -> Result<Self> {
        ensure!(
            !settle.is_zero(),
            "persistence settle interval must be positive"
        );
        let (inscriptions, work) = superseding_channel::<Inscription<T>>();
        let (publish, outcomes) = superseding_channel();
        let wake = NativeWake::from_context(ctx);
        let thread = thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                while let Ok(inscription) = work.recv() {
                    let sequence = inscription.sequence;
                    let result = write(inscription.snapshot).map_err(|error| format!("{error:#}"));
                    if let Some(receipt) = inscription.receipt {
                        let _received = receipt.send(result);
                    } else {
                        let outcome = result.map_or_else(
                            |message| ScribeOutcome::Fault { sequence, message },
                            |()| ScribeOutcome::Saved { sequence },
                        );
                        if publish.offer(outcome).is_err() {
                            break;
                        }
                        let _woken = wake.request_foreground_repaint();
                    }
                }
            })
            .context("spawn persistence scribe")?;
        Ok(Self {
            settle,
            dirty: None,
            sequence: 0,
            inscriptions: Some(inscriptions),
            outcomes,
            thread: Some(thread),
        })
    }

    /// Restart the settlement clock after a source-state mutation.
    pub fn mark(&mut self) {
        self.dirty = Some(Instant::now());
    }

    /// Cancel a dirty epoch after the source state returns to its durable value.
    pub fn clear(&mut self) {
        self.dirty = None;
    }

    /// Report when the current dirty epoch may be inscribed.
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        self.dirty.and_then(|dirty| dirty.checked_add(self.settle))
    }

    /// Submit the settled snapshot without waiting.
    ///
    /// Returns the minted sequence only when a matured dirty epoch was submitted.
    pub fn tend(&mut self, now: Instant, snapshot: impl FnOnce() -> T) -> Result<Option<u64>> {
        if self.deadline().is_none_or(|deadline| deadline > now) {
            return Ok(None);
        }
        self.submit(snapshot()).map(Some)
    }

    /// Submit a snapshot immediately without waiting for settlement or I/O.
    pub fn submit(&mut self, snapshot: T) -> Result<u64> {
        let sequence = self.mint_sequence()?;
        self.dirty = None;
        self.inscriptions
            .as_ref()
            .context("persistence scribe has retired")?
            .offer(Inscription {
                sequence,
                snapshot,
                receipt: None,
            })
            .map_err(|_| anyhow!("persistence scribe has fallen"))?;
        Ok(sequence)
    }

    /// Receive the latest background outcome without waiting.
    #[must_use]
    pub fn take_outcome(&self) -> Option<ScribeOutcome> {
        self.outcomes.try_recv().ok()
    }

    /// Inscribe one final snapshot and wait for its durable result.
    ///
    /// This is intended only for orderly application retirement.
    pub fn flush(&mut self, snapshot: T) -> Result<()> {
        self.dirty = None;
        let sequence = self.mint_sequence()?;
        let (receipt, result) = bounded(1);
        let inscription = Inscription {
            sequence,
            snapshot,
            receipt: Some(receipt),
        };
        let superseded = self
            .inscriptions
            .as_ref()
            .context("persistence scribe has retired")?
            .offer(inscription)
            .map_err(|_| anyhow!("persistence scribe has fallen"))?;
        if let Some(receipt) = superseded.and_then(|prior| prior.receipt) {
            let _received = receipt.send(Err(
                "synchronous persistence inscription was superseded".to_owned()
            ));
        }
        result
            .recv()
            .context("persistence scribe fell before its receipt")?
            .map_err(anyhow::Error::msg)
    }

    fn mint_sequence(&mut self) -> Result<u64> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .context("persistence sequence exhausted")?;
        Ok(self.sequence)
    }
}

impl<T> Drop for SettledScribe<T> {
    fn drop(&mut self) {
        drop(self.inscriptions.take());
        if let Some(thread) = self.thread.take() {
            let _joined = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_coalescence_and_retirement_cross_the_worker_boundary() -> Result<()> {
        let (publish, written) = bounded(3);
        let (announce, started) = bounded(1);
        let (release, gate) = bounded(1);
        let ctx = egui::Context::default();
        let mut first = true;
        let mut scribe = SettledScribe::spawn(
            "scribe-test",
            &ctx,
            Duration::from_millis(400),
            move |value| {
                if first {
                    first = false;
                    announce.send(()).context("announce claimed inscription")?;
                    gate.recv().context("release claimed inscription")?;
                }
                publish.send(value).context("publish written value")
            },
        )?;
        scribe.mark();
        let deadline = scribe.deadline().context("dirty deadline")?;
        let premature = deadline
            .checked_sub(Duration::from_nanos(1))
            .context("settlement deadline has a predecessor")?;
        assert_eq!(scribe.tend(premature, || 7)?, None);
        assert_eq!(scribe.tend(deadline, || 7)?, Some(1));
        started.recv_timeout(Duration::from_secs(1))?;
        assert_eq!(scribe.submit(8)?, 2);
        assert_eq!(scribe.submit(9)?, 3);
        release.send(())?;
        assert_eq!(written.recv_timeout(Duration::from_secs(1))?, 7);
        assert_eq!(written.recv_timeout(Duration::from_secs(1))?, 9);
        assert!(
            written.try_recv().is_err(),
            "superseded snapshot was written"
        );

        let outcome = (0..100).find_map(|_| {
            let outcome = scribe.take_outcome();
            if outcome.is_none() {
                thread::sleep(Duration::from_millis(1));
            }
            outcome
        });
        assert!(matches!(
            outcome,
            Some(ScribeOutcome::Saved { sequence: 3 })
        ));

        scribe.flush(10)?;
        assert_eq!(written.recv_timeout(Duration::from_secs(1))?, 10);
        Ok(())
    }
}
