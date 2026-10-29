use futures_util::future::LocalBoxFuture;
use futures_util::stream::{FuturesUnordered, StreamExt};
use tokio::sync::{mpsc, OwnedSemaphorePermit};
use wtransport::error::ConnectionError;
use wtransport::{Connection, RecvStream, SendStream};

use crate::log;
use crate::log::hex;
use crate::Store;
use crate::Payload::Request::Request;
use crate::Payload::Response::Response;

// ---------------------------------------------------------------------------
// Workers: WORKERS long-lived tasks share all connections between them.
// Connection in slot s belongs to worker s % WORKERS (100 slots → 10 each).
// ---------------------------------------------------------------------------

pub const WORKERS: usize = 10;

/// Largest payload a frame can carry (`len` is one byte).
const MAX_FRAME: usize = u8::MAX as usize;

/// A client that holds a permit and a slot; sent from the interface to a worker.
pub struct Admitted {
    pub slot: usize,
    pub id: u64,
    pub conn: Connection,
    /// Never read, only held: dropping it when the client leaves admits the next in line.
    #[allow(dead_code)]
    pub permit: OwnedSemaphorePermit,
}

/// One bidirectional stream and its buffers, owned by the worker.
struct Stream {
    slot: usize,
    id: u64,
    send: SendStream,
    recv: RecvStream,
    buf: [u8; MAX_FRAME],
    /// Pending reply frame [len][payload]; empty when there is nothing to send.
    out: Vec<u8>,
}

/// What a worker waits for. Every connection and stream it owns has exactly
/// one of these futures pending at any time.
enum Event {
    /// The client opened a stream, or its connection ended (Err).
    NewStream(Admitted, Result<(SendStream, RecvStream), ConnectionError>),
    /// A request frame of `n` bytes is in `buf`, or the stream closed (None).
    Frame(Stream, Option<usize>),
}

fn next_stream(client: Admitted) -> LocalBoxFuture<'static, Event> {
    Box::pin(async move {
        let result = client.conn.accept_bi().await;
        Event::NewStream(client, result)
    })
}

/// Sends the previous reply (if any), then reads the next request frame.
/// Writing here, not in the worker loop, means a slow client only holds up
/// its own stream, never the other connections of the same worker.
fn next_frame(mut s: Stream) -> LocalBoxFuture<'static, Event> {
    Box::pin(async move {
        if !s.out.is_empty() {
            if s.send.write_all(&s.out).await.is_err() {
                return Event::Frame(s, None);
            }
            s.out.clear();
        }
        let mut len = [0u8; 1];
        if s.recv.read_exact(&mut len).await.is_err() {
            return Event::Frame(s, None);
        }
        let n = len[0] as usize;
        if s.recv.read_exact(&mut s.buf[..n]).await.is_err() {
            return Event::Frame(s, None);
        }
        Event::Frame(s, Some(n))
    })
}

/// One worker task: waits on all of its connections and streams at once and
/// handles whichever is ready. Ends when the inbox closes and nothing is left.
pub async fn run(w: usize, mut inbox: mpsc::UnboundedReceiver<Admitted>, store: Store) {
    let mut pending: FuturesUnordered<LocalBoxFuture<'static, Event>> = FuturesUnordered::new();
    let mut inbox_open = true;

    loop {
        tokio::select! {
            new = inbox.recv(), if inbox_open => match new {
                Some(client) => {
                    log!("[w{w}] took slot {} (client {})", client.slot, client.id);
                    pending.push(next_stream(client));
                }
                None => inbox_open = false,
            },
            Some(event) = pending.next(), if !pending.is_empty() => match event {
                Event::NewStream(client, Ok((send, recv))) => {
                    pending.push(next_frame(Stream {
                        slot: client.slot,
                        id: client.id,
                        send,
                        recv,
                        buf: [0; MAX_FRAME],
                        out: Vec::with_capacity(1 + 32),
                    }));
                    pending.push(next_stream(client));
                }
                Event::NewStream(client, Err(_)) => {
                    if let Some(c) = store.borrow_mut().remove(client.slot, client.id) {
                        log!("[w{w} slot {}] client {} from {} disconnected", client.slot, c.id, c.remote);
                    }
                    // `client.permit` drops here: the next client in line gets in.
                }
                Event::Frame(mut s, Some(n)) => {
                    handle_frame(w, &store, &mut s, n);
                    pending.push(next_frame(s));
                }
                Event::Frame(_, None) => {} // stream closed by the client
            },
            else => break,
        }
    }
    log!("[w{w}] stopped");
}

/// Decodes the request in `s.buf[..n]` and puts the reply frame in `s.out`.
/// A zero-length reply means the request could not be decoded.
fn handle_frame(w: usize, store: &Store, s: &mut Stream, n: usize) {
    let payload = &s.buf[..n];
    let reply = match Request::decode(payload) {
        Some(req) => {
            log!("[w{w} slot {}] -> {req:?}", s.slot);
            match handle_request(store, s.slot, s.id, req) {
                Some(resp) => resp.encode(),
                None => return, // client already gone; nothing to send
            }
        }
        None => {
            log!("[w{w} slot {}] undecodable payload {}", s.slot, hex(payload));
            Vec::new()
        }
    };
    s.out.push(reply.len() as u8);
    s.out.extend_from_slice(&reply);
}

/// Plain synchronous function: borrows the store, updates it, returns.
fn handle_request(store: &Store, slot: usize, id: u64, req: Request) -> Option<Response> {
    let mut store = store.borrow_mut();
    let client = store.get_mut(slot, id)?;
    Some(match req {
        Request::GetFen => Response::Fen(client.fen.clone()),
        Request::SetFen(fen) => {
            client.fen = fen;
            Response::Fen(client.fen.clone())
        }
        // TODO: legal moves / move validation from the engine crate.
        Request::Select(square) => Response::Select(square),
        Request::Move(mv) => Response::Move(mv),
    })
}
