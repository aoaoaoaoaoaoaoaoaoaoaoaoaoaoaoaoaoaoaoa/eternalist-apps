//! Opt-in responsiveness evidence for the native application shell.
//!
//! `ETERNALIST_TRACE=/path/to/trace.json` records production-path spans in
//! Chrome trace format. Instrumentation is dormant when the variable is
//! absent; call sites then reduce to `tracing`'s disabled-span fast path.

use anyhow::{Context as _, Result};
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded};
use std::{
    fs::File,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tracing_subscriber::{
    Layer as _, filter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

/// Environment variable naming the Chrome-trace output path.
pub const TRACE_PATH_ENV: &str = "ETERNALIST_TRACE";
/// Optional positive trace duration in seconds; requires [`TRACE_PATH_ENV`].
pub const TRACE_SECONDS_ENV: &str = "ETERNALIST_TRACE_SECONDS";
const TRACE_TARGET_ROOT: &str = "eternalist";

/// Per-frame admission law for worker results installed on the event-loop thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrainBudget {
    items: usize,
    wall: Duration,
}

/// Live item and wall-clock allowance minted by a [`DrainBudget`].
#[derive(Clone, Copy, Debug)]
pub struct Drain {
    remaining: usize,
    wall: Duration,
    begun: Instant,
}

/// The producer half of a one-slot, latest-demand-wins mailbox.
///
/// [`Self::offer`] never waits. When one demand is queued but not yet claimed,
/// a newer demand supplants it. Exactly one producer and one consumer must own
/// the two halves; cloning either role would make supersession ambiguous.
pub struct SupersedingSender<T> {
    channel: Sender<T>,
    pending: Receiver<T>,
    consumer_alive: Arc<AtomicBool>,
}

/// The consumer half of a [`superseding_channel`].
pub struct SupersedingReceiver<T> {
    channel: Receiver<T>,
    consumer_alive: Arc<AtomicBool>,
}

/// Forge a nonblocking one-slot mailbox whose pending demand may be superseded.
///
/// This is for idempotent or serial-tagged work where completing an older
/// queued demand has no value once a newer one exists. Work already claimed by
/// the consumer is never cancelled.
#[must_use]
pub fn superseding_channel<T>() -> (SupersedingSender<T>, SupersedingReceiver<T>) {
    let (channel, receiver) = bounded(1);
    let consumer_alive = Arc::new(AtomicBool::new(true));
    (
        SupersedingSender {
            channel,
            pending: receiver.clone(),
            consumer_alive: Arc::clone(&consumer_alive),
        },
        SupersedingReceiver {
            channel: receiver,
            consumer_alive,
        },
    )
}

impl DrainBudget {
    /// Define a nonempty per-frame item and wall-clock ceiling.
    #[must_use]
    pub const fn new(items: usize, wall: Duration) -> Self {
        assert!(items > 0, "a drain budget must admit at least one item");
        assert!(!wall.is_zero(), "a drain budget must admit positive time");
        Self { items, wall }
    }

    /// Mint one allowance for the current frame.
    #[must_use]
    pub fn arm(self) -> Drain {
        Drain {
            remaining: self.items,
            wall: self.wall,
            begun: Instant::now(),
        }
    }
}

impl Drain {
    /// Receive one item only while both ceilings remain open.
    ///
    /// The receiver is not called after the allowance closes. A successful
    /// receipt consumes one item; an empty receiver does not.
    pub fn take<T>(&mut self, receive: impl FnOnce() -> Option<T>) -> Option<T> {
        if self.remaining == 0 || self.begun.elapsed() >= self.wall {
            return None;
        }
        let item = receive()?;
        self.remaining -= 1;
        Some(item)
    }

    /// Try one crossbeam receiver under this allowance.
    pub fn receive<T>(&mut self, receiver: &Receiver<T>) -> Option<T> {
        self.take(|| receiver.try_recv().ok())
    }
}

impl<T> SupersedingSender<T> {
    /// Queue `demand` without waiting, returning any older pending demand.
    ///
    /// An error returns the offered demand after the consumer has gone away.
    pub fn offer(&self, mut demand: T) -> Result<Option<T>, T> {
        let mut superseded = None;
        loop {
            if !self.consumer_alive.load(Ordering::Acquire) {
                return Err(demand);
            }
            match self.channel.try_send(demand) {
                Ok(()) => {
                    if self.consumer_alive.load(Ordering::Acquire) {
                        return Ok(superseded);
                    }
                    return match self.pending.try_recv() {
                        Ok(orphaned) => Err(orphaned),
                        Err(TryRecvError::Empty | TryRecvError::Disconnected) => Ok(superseded),
                    };
                }
                Err(TrySendError::Full(returned)) => {
                    demand = returned;
                    match self.pending.try_recv() {
                        Ok(prior) => superseded = Some(prior),
                        Err(TryRecvError::Empty) => {}
                        Err(TryRecvError::Disconnected) => return Err(demand),
                    }
                }
                Err(TrySendError::Disconnected(returned)) => return Err(returned),
            }
        }
    }
}

impl<T> SupersedingReceiver<T> {
    /// Borrow the underlying channel for `crossbeam_channel::select!`.
    #[must_use]
    pub const fn channel(&self) -> &Receiver<T> {
        &self.channel
    }

    /// Wait for the next surviving demand.
    pub fn recv(&self) -> Result<T, crossbeam_channel::RecvError> {
        self.channel.recv()
    }

    /// Try to claim the next surviving demand without waiting.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.channel.try_recv()
    }
}

impl<T> Drop for SupersedingReceiver<T> {
    fn drop(&mut self) {
        self.consumer_alive.store(false, Ordering::Release);
    }
}

/// Owns the trace writer until the native application terminates.
pub struct TraceGuard {
    writer: Option<tracing_chrome::FlushGuard>,
}

impl TraceGuard {
    /// Install the Eternalist trace collector when `ETERNALIST_TRACE` names a file.
    pub fn arm() -> Result<Self> {
        let Some(path) = std::env::var_os(TRACE_PATH_ENV) else {
            return Ok(Self { writer: None });
        };
        let path = Path::new(&path);
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create trace directory {}", parent.display()))?;
        }
        let file = File::create(path)
            .with_context(|| format!("create responsiveness trace {}", path.display()))?;
        let (layer, writer) = tracing_chrome::ChromeLayerBuilder::new()
            .writer(file)
            .include_args(true)
            .include_locations(false)
            .build();
        let eternalist_only = filter::filter_fn(|metadata| {
            metadata.target() == TRACE_TARGET_ROOT
                || metadata
                    .target()
                    .strip_prefix(TRACE_TARGET_ROOT)
                    .is_some_and(|suffix| suffix.starts_with("::"))
        });
        tracing_subscriber::registry()
            .with(layer.with_filter(eternalist_only))
            .try_init()
            .context("install responsiveness trace collector")?;
        eprintln!("responsiveness trace: {}", path.display());
        Ok(Self {
            writer: Some(writer),
        })
    }

    /// Push all completed events to the trace file without ending collection.
    pub fn flush(&self) {
        if let Some(writer) = &self.writer {
            writer.flush();
        }
    }
}

pub(crate) fn deadline() -> Result<Option<Instant>> {
    let Some(raw) = std::env::var_os(TRACE_SECONDS_ENV) else {
        return Ok(None);
    };
    let seconds = raw
        .to_str()
        .context("ETERNALIST_TRACE_SECONDS is not Unicode")?
        .parse::<u64>()
        .context("ETERNALIST_TRACE_SECONDS is not a positive integer")?;
    anyhow::ensure!(seconds > 0, "ETERNALIST_TRACE_SECONDS must be positive");
    anyhow::ensure!(
        std::env::var_os(TRACE_PATH_ENV).is_some(),
        "ETERNALIST_TRACE_SECONDS requires ETERNALIST_TRACE"
    );
    Ok(Some(Instant::now() + Duration::from_secs(seconds)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superseding_mailbox_preserves_latest_pending_claimed_work_and_consumer_death() {
        let (sender, receiver) = superseding_channel();
        assert!(matches!(sender.offer(1), Ok(None)));
        assert!(matches!(sender.offer(2), Ok(Some(1))));
        assert_eq!(receiver.try_recv(), Ok(2));

        assert!(matches!(sender.offer(3), Ok(None)));
        assert_eq!(receiver.try_recv(), Ok(3));
        assert!(matches!(sender.offer(4), Ok(None)));
        assert_eq!(receiver.try_recv(), Ok(4));

        drop(receiver);
        assert_eq!(sender.offer(5), Err(5));
    }
}
