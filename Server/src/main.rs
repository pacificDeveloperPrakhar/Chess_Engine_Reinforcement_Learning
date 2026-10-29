use anyhow::Result;
use futures_util::stream::{FuturesUnordered, StreamExt};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::{spawn_local, LocalSet};
use wtransport::endpoint::IncomingSession;
use wtransport::{Endpoint, Identity, ServerConfig};

pub mod Payload;
mod admission;
mod connections;
mod log;
mod worker;

use admission::{Admission, MAX_WAITING};
use connections::{Client, Connections, MAX_CONNECTIONS, START_FEN};
use worker::{Admitted, WORKERS};

const PORT: u16 = 4433;

/// Shared by every task. Single thread, so Rc + RefCell instead of Arc + Mutex.
/// Never hold a borrow across an `.await`.
pub type Store = Rc<RefCell<Connections>>;

/// Hands out a unique id to every session.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

// ---------------------------------------------------------------------------
// main: starts the "interface" thread, which runs everything, and waits for it
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    log!("[main] starting up");

    let interface = std::thread::Builder::new()
        .name("interface".into())
        .spawn(|| -> Result<()> {
            // current_thread: no worker pool, every task runs on this thread.
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

            // LocalSet lets tasks share !Send data (the Rc<RefCell<..>> store).
            LocalSet::new().block_on(&rt, run())
        })?;

    let result = interface.join().map_err(|_| anyhow::anyhow!("interface thread panicked"))?;
    log!("[main] interface thread finished");
    result
}

// ---------------------------------------------------------------------------
// Interface: accepts clients and hands each one to a worker
// ---------------------------------------------------------------------------

async fn run() -> Result<()> {
    let endpoint = bind_endpoint(PORT)?;
    let store: Store = Rc::new(RefCell::new(Connections::new()));
    let admission = Rc::new(Admission::new(MAX_CONNECTIONS));

    // The fixed pool: WORKERS tasks, started once, never more.
    let workers: Vec<_> = (0..WORKERS)
        .map(|w| {
            let (tx, rx) = mpsc::unbounded_channel();
            spawn_local(worker::run(w, rx, store.clone()));
            tx
        })
        .collect();
    log!(
        "[interface] listening on https://localhost:{PORT}: {MAX_CONNECTIONS} slots, \
         {WORKERS} workers, {MAX_WAITING} waiting"
    );

    // Handshakes and clients waiting in line, all polled from this one future.
    let mut admitting = FuturesUnordered::new();
    loop {
        tokio::select! {
            incoming = endpoint.accept() => {
                admitting.push(admit(incoming, store.clone(), admission.clone()));
            }
            Some(admitted) = admitting.next(), if !admitting.is_empty() => {
                if let Some(client) = admitted {
                    let _ = workers[client.slot % WORKERS].send(client);
                }
            }
            _ = tokio::signal::ctrl_c() => {
                log!("[interface] Ctrl-C received, shutting down");
                break;
            }
        }
    }

    admission.close();
    store.borrow().close_all();
    Ok(())
}

fn bind_endpoint(port: u16) -> Result<Endpoint<wtransport::endpoint::endpoint_side::Server>> {
    let identity = Identity::self_signed(["localhost", "127.0.0.1", "::1"])?;
    let digest = identity.certificate_chain().as_slice()[0].hash();
    let hex_digest: String = digest.as_ref().iter().map(|b| format!("{b:02x}")).collect();
    log!("[interface] cert sha-256: {hex_digest}");
    log!("[interface] ^ give this to the browser via serverCertificateHashes");

    let config = ServerConfig::builder()
        .with_bind_default(port)
        .with_identity(identity)
        .keep_alive_interval(Some(Duration::from_secs(3)))
        .build();
    Ok(Endpoint::server(config)?)
}

/// Handshake → permit (at once, or after waiting in line) → slot.
/// Returns None if the client was refused or the handshake failed.
async fn admit(incoming: IncomingSession, store: Store, admission: Rc<Admission>) -> Option<Admitted> {
    let request = match incoming.await {
        Ok(request) => request,
        Err(e) => {
            log!("[interface] handshake failed: {e}");
            return None;
        }
    };

    // Free slot: enter at once. Otherwise wait in line (CONNECT stays unanswered).
    let full = store.borrow().live() >= MAX_CONNECTIONS;
    if full {
        log!("[queue] all {MAX_CONNECTIONS} slots taken, waiting in line ({} ahead)", admission.waiting());
    }
    let permit = match admission.enter().await {
        Ok(permit) => permit,
        Err(reason) => {
            log!("[queue] refused with 429: {reason:?}");
            request.too_many_requests().await;
            return None;
        }
    };
    if full {
        log!("[queue] slot freed, admitting ({} still waiting)", admission.waiting());
    }

    let conn = match request.accept().await {
        Ok(conn) => conn,
        Err(e) => {
            log!("[interface] accept failed: {e}");
            return None; // permit drops here
        }
    };
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let remote = conn.remote_address();

    let inserted = store.borrow_mut().insert(Client {
        id,
        conn: conn.clone(),
        remote,
        fen: START_FEN.to_string(),
    });
    // One permit per slot, so holding a permit guarantees a free slot.
    let Some(slot) = inserted else {
        unreachable!("permit held but no free slot");
    };
    log!(
        "[slot {slot}] client {id} connected from {remote} → worker {} ({} live)",
        slot % WORKERS,
        store.borrow().live()
    );

    Some(Admitted { slot, id, conn, permit })
}
