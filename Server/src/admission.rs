use std::cell::Cell;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// ---------------------------------------------------------------------------
// Admission: one permit per slot + a bounded FIFO waiting line
// ---------------------------------------------------------------------------

/// How many clients may wait for a free slot at the same time.
pub const MAX_WAITING: usize = 50;

/// How long a waiting client is held before it is refused.
pub const MAX_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum Rejected {
    /// MAX_WAITING clients are already in line.
    LineFull,
    /// Waited MAX_WAIT without a slot freeing up.
    TimedOut,
    /// Server is shutting down.
    Closed,
}

/// Holding the permit = owning a slot. Dropping it hands the slot straight to
/// the client that has waited longest (tokio's Semaphore is FIFO-fair).
pub struct Admission {
    permits: Arc<Semaphore>,
    waiting: Cell<usize>,
}

impl Admission {
    pub fn new(slots: usize) -> Self {
        Self { permits: Arc::new(Semaphore::new(slots)), waiting: Cell::new(0) }
    }

    pub fn waiting(&self) -> usize {
        self.waiting.get()
    }

    /// Returns at once if a slot is free, otherwise waits in line.
    pub async fn enter(&self) -> Result<OwnedSemaphorePermit, Rejected> {
        // Fails while others are waiting, so nobody can skip the line.
        if let Ok(permit) = self.permits.clone().try_acquire_owned() {
            return Ok(permit);
        }
        if self.waiting.get() >= MAX_WAITING {
            return Err(Rejected::LineFull);
        }

        self.waiting.set(self.waiting.get() + 1);
        let result = tokio::time::timeout(MAX_WAIT, self.permits.clone().acquire_owned()).await;
        self.waiting.set(self.waiting.get() - 1);

        match result {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(_)) => Err(Rejected::Closed),
            Err(_) => Err(Rejected::TimedOut),
        }
    }

    /// Wakes every waiting client with `Rejected::Closed`.
    pub fn close(&self) {
        self.permits.close();
    }
}
