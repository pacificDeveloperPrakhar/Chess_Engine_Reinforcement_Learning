//! Smoke + performance test for the chess server (WebTransport over QUIC).
//!
//!   cargo run --release --example smoke_client -- <cert-sha256-hex-from-server-log>
//!
//! Phases (every step is logged):
//!   1. CLIENTS clients connect; each runs every request kind and checks the reply.
//!   2. Burst: all CLIENTS send one Move at the same instant (timed).
//!   3. WAITING more clients connect; all slots are full, so they must wait in line.
//!   4. One more client must be refused at once (line full).
//!   5. The first CLIENTS clients leave; every waiting client must get in, in turn.

use anyhow::{ensure, Result};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Barrier;
use wtransport::tls::Sha256Digest;
use wtransport::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, VarInt};

// The server's own codec, so requests are built exactly the way the server reads them.
#[path = "../src/Payload/mod.rs"]
#[allow(non_snake_case, dead_code)]
mod Payload;
use Payload::Move;
use Payload::Request::Request;
use Payload::Response::Response;

const CLIENTS: usize = 100; // = MAX_CONNECTIONS in src/connections.rs
const WAITING: usize = 50; // = MAX_WAITING in src/admission.rs

const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
const AFTER_E4: &str = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR";
const E2E4: Move = Move { from: 1 << 12, to: 1 << 28, role: 0 };

// ---------------------------------------------------------------------------
// Logging: "[  12.34ms] [client 3  ] message"
// ---------------------------------------------------------------------------

fn ms() -> f64 {
    static T0: OnceLock<Instant> = OnceLock::new();
    T0.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

macro_rules! log {
    ($tag:expr, $($arg:tt)*) => {
        println!("[{:>9.2}ms] [{:<10}] {}", ms(), $tag, format!($($arg)*))
    };
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Request dispatcher: one bidirectional stream, frames = [len: u8][payload]
// ---------------------------------------------------------------------------

struct Dispatcher {
    tag: String,
    send: SendStream,
    recv: RecvStream,
    frame: Vec<u8>, // reused for every outgoing frame
}

impl Dispatcher {
    async fn open(tag: &str, conn: &Connection) -> Result<Self> {
        let (send, recv) = conn.open_bi().await?.await?;
        log!(tag, "bidirectional stream opened");
        Ok(Self { tag: tag.to_string(), send, recv, frame: Vec::with_capacity(1 + 32) })
    }

    /// Frames raw payload bytes, sends them, and returns the raw reply payload.
    async fn exchange(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        self.frame.clear();
        self.frame.push(payload.len() as u8);
        self.frame.extend_from_slice(payload);

        let t = Instant::now();
        self.send.write_all(&self.frame).await?;
        let mut len = [0u8; 1];
        self.recv.read_exact(&mut len).await?;
        let mut reply = vec![0u8; len[0] as usize];
        self.recv.read_exact(&mut reply).await?;

        log!(
            &self.tag,
            "   wire: sent [{:02x}] {} | got [{:02x}] {} | {:.2?}",
            payload.len(),
            hex(payload),
            reply.len(),
            hex(&reply),
            t.elapsed()
        );
        Ok(reply)
    }

    /// Encodes a Request, sends it, decodes the Response (None = error frame).
    async fn request(&mut self, req: &Request) -> Result<Option<Response>> {
        log!(&self.tag, "-> {req:?}");
        let reply = self.exchange(&req.encode()).await?;
        let resp = Response::decode(&reply);
        log!(&self.tag, "<- {resp:?}");
        Ok(resp)
    }

    /// Sends a request and fails the test if the reply is not `want`.
    async fn expect(&mut self, req: Request, want: Response) -> Result<()> {
        let got = self.request(&req).await?;
        ensure!(got.as_ref() == Some(&want), "{}: {req:?} expected {want:?}, got {got:?}", self.tag);
        log!(&self.tag, "   ok");
        Ok(())
    }
}

async fn connect(hash: [u8; 32]) -> Result<Connection> {
    let config = ClientConfig::builder()
        .with_bind_default()
        .with_server_certificate_hashes([Sha256Digest::new(hash)])
        .build();
    Ok(Endpoint::client(config)?.connect("https://localhost:4433").await?)
}

// ---------------------------------------------------------------------------
// Phase 1 + 2: a connected client runs every request kind, then the burst
// ---------------------------------------------------------------------------

struct Burst {
    conn: Connection,
    round_trip: Duration,  // this client's own send -> reply
    since_start: Duration, // shared start instant -> this client's reply
}

async fn client(n: usize, hash: [u8; 32], barrier: Arc<Barrier>, start: Arc<OnceLock<Instant>>) -> Result<Burst> {
    let tag = format!("client {n}");
    log!(&tag, "connecting");
    let t = Instant::now();
    let conn = connect(hash).await?;
    log!(&tag, "connected in {:.2?} (QUIC + TLS 1.3 + HTTP/3 CONNECT)", t.elapsed());
    let mut d = Dispatcher::open(&tag, &conn).await?;

    d.expect(Request::GetFen, Response::Fen(START_FEN.into())).await?;
    d.expect(Request::SetFen(AFTER_E4.into()), Response::Fen(AFTER_E4.into())).await?;
    // The board is kept in this client's slot on the server.
    d.expect(Request::GetFen, Response::Fen(AFTER_E4.into())).await?;
    d.expect(Request::Select(E2E4.from), Response::Select(E2E4.from)).await?;
    d.expect(Request::Move(E2E4), Response::Move(E2E4)).await?;

    log!(&tag, "-> raw payload aa bb (length 2 is not a request)");
    let reply = d.exchange(&[0xAA, 0xBB]).await?;
    ensure!(reply.is_empty(), "{tag}: bad request got {} bytes", reply.len());
    log!(&tag, "<- error frame (0 bytes)");
    log!(&tag, "   ok");

    log!(&tag, "all checks passed, waiting at the barrier");
    barrier.wait().await;

    let shared_start = *start.get_or_init(Instant::now);
    let sent = Instant::now();
    let got = d.request(&Request::Move(E2E4)).await?;
    let (round_trip, since_start) = (sent.elapsed(), shared_start.elapsed());
    ensure!(got == Some(Response::Move(E2E4)), "{tag}: burst reply {got:?}");
    Ok(Burst { conn, round_trip, since_start })
}

// ---------------------------------------------------------------------------
// Phase 3 + 5: a client that must wait in line, then gets served and leaves
// ---------------------------------------------------------------------------

async fn waiter(n: usize, hash: [u8; 32]) -> Result<Instant> {
    let tag = format!("waiter {n}");
    log!(&tag, "connecting (slots full, expect to wait in line)");
    let t = Instant::now();
    let conn = connect(hash).await?;
    log!(&tag, "admitted after {:.2?} in line", t.elapsed());

    let mut d = Dispatcher::open(&tag, &conn).await?;
    d.expect(Request::GetFen, Response::Fen(START_FEN.into())).await?;
    let served = Instant::now();

    conn.close(VarInt::from_u32(0), b"done");
    log!(&tag, "left; its slot goes to the next in line");
    Ok(served)
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let hex_arg = std::env::args().nth(1).unwrap_or_default();
    ensure!(hex_arg.len() == 64, "usage: smoke_client <cert-sha256-hex from the server log>");
    let mut hash = [0u8; 32];
    for (i, b) in hash.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex_arg[i * 2..i * 2 + 2], 16)?;
    }
    ms(); // start the log clock

    log!("main", "=== phase 1: {CLIENTS} clients connect and check every request kind");
    let barrier = Arc::new(Barrier::new(CLIENTS));
    let start = Arc::new(OnceLock::new());
    let tasks: Vec<_> = (1..=CLIENTS)
        .map(|n| tokio::spawn(client(n, hash, barrier.clone(), start.clone())))
        .collect();
    let mut bursts = Vec::with_capacity(CLIENTS);
    for t in tasks {
        bursts.push(t.await??);
    }

    log!("main", "=== phase 2: burst results ({CLIENTS} Moves sent at the same instant)");
    let mut rts: Vec<_> = bursts.iter().map(|b| b.round_trip).collect();
    rts.sort();
    let all_done = bursts.iter().map(|b| b.since_start).max().unwrap();
    log!("main", "round trip p50 {:.2?} | max {:.2?}", rts[rts.len() / 2], rts[rts.len() - 1]);
    log!("main", "all {CLIENTS} replies received within {all_done:.2?} of the start");

    log!("main", "=== phase 3: {WAITING} more clients connect while all {CLIENTS} slots are full");
    let waiters: Vec<_> = (1..=WAITING).map(|n| tokio::spawn(waiter(n, hash))).collect();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let early = waiters.iter().filter(|w| w.is_finished()).count();
    log!("main", "after 500ms: {early} of {WAITING} waiters admitted (expect 0)");
    ensure!(early == 0, "waiters got in while every slot was taken");

    log!("main", "=== phase 4: one more client while the line is full");
    let t = Instant::now();
    match connect(hash).await {
        Ok(_) => anyhow::bail!("client {} was accepted, expected 429", CLIENTS + WAITING + 1),
        Err(e) => log!("main", "client {} refused in {:.2?}: {e}", CLIENTS + WAITING + 1, t.elapsed()),
    }

    log!("main", "=== phase 5: the {CLIENTS} first clients leave");
    let freed = Instant::now();
    for b in &bursts {
        b.conn.close(VarInt::from_u32(0), b"leaving");
    }
    let mut last = freed;
    for w in waiters {
        last = last.max(tokio::time::timeout(Duration::from_secs(20), w).await???);
    }
    log!(
        "main",
        "all {WAITING} waiters admitted and served within {:.2?}",
        last - freed
    );

    log!("main", "=== ALL CHECKS PASSED");
    Ok(())
}
