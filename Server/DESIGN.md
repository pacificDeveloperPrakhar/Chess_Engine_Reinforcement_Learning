# Server Design Reference

WebTransport over QUIC · one "interface" thread · 10 worker tasks · 100 connections · waiting line

Reference: <https://docs.rs/tokio/latest/tokio/runtime/index.html>

## 1. Threads, the interface and the workers

```
process
│
├── main thread ─────────── starts "interface", then join()s it (does nothing else)
│
└── "interface" thread ──── tokio current_thread runtime + LocalSet
        │                   (everything below runs on this one thread)
        │
        ├── interface (the main future) ......... accept loop + every pending handshake
        │      │                                  and every client waiting in line,
        │      │                                  all in one FuturesUnordered
        │      ├── Admission ...... Semaphore(100 permits) + waiting line (max 50, max 30 s)
        │      └── Connections .... Vec<Option<Client>> with 100 slots, on the heap
        │                           Rc<RefCell<..>>, no locks (single thread)
        │
        │      admitted client in slot s ──mpsc──► worker (s % 10)
        │
        └── worker 0 … worker 9 ... 10 tasks, spawned once at startup
               each owns ≤ 10 connections (slots w, w+10, w+20, … w+90)
               and waits on all of them at once (FuturesUnordered):
                 • "next stream" on each of its connections
                 • "write reply, then read next frame" on each of its streams
```

| Part | File | Job |
| --- | --- | --- |
| `main` | `src/main.rs` | Starts the `interface` thread and waits for it |
| `run` (interface) | `src/main.rs` | Binds the endpoint, starts the 10 workers, accepts clients, hands each admitted client to its worker, Ctrl-C shutdown |
| `admit` | `src/main.rs` | Handshake → permit (now, or after waiting in line) → slot |
| `worker::run` | `src/worker.rs` | One of the 10 workers: waits on its connections and streams, handles whichever is ready |
| `handle_frame` / `handle_request` | `src/worker.rs` | Plain synchronous code: decode → read/update the client's slot → encode reply |
| `Admission` | `src/admission.rs` | 100 permits + bounded FIFO waiting line |
| `Connections` | `src/connections.rs` | The 100-slot heap `Vec` of clients |
| `Payload` | `src/Payload/` | Request/Response encode + decode (shared with the test) |

### The concurrency model

- **One thread, 11 tasks:** the interface (the future passed to `block_on`) and
  the 10 workers. The task count does not grow with the number of clients.
  (`wtransport`'s QUIC library runs its own internal driver tasks on the same
  thread; those are not ours.)
- **Each worker multiplexes.** A worker keeps one pending future per
  connection ("wait for the next stream") and one per stream ("send the last
  reply, then read the next request"). `FuturesUnordered` wakes the worker for
  whichever is ready.
- **Writes happen inside the per-stream future**, so a slow client only holds
  up its own stream, never the other connections of its worker.
- **Asynchronous, not parallel.** Every `.await` that has to wait hands the
  thread to another task, so all 100 connections make progress concurrently.
  Only one piece of code runs at any instant.
- **No locks.** `handle_request` has no `.await`, so each request runs to
  completion before the next one starts, and only this thread touches the store.
- **What would freeze everyone:** slow synchronous work in `handle_request`
  (for example a deep engine search). That belongs on `spawn_blocking`.

Every slot stores a unique client `id`. Lookups check slot + id, so a stream
from a client that has left can never change the next client in that slot.

## 2. How a client gets in

```
QUIC + TLS handshake, HTTP/3 CONNECT            (inside the interface)
            │
            ▼
   free permit? ──yes──────────────────────────────┐
            │ no                                   │
            ▼                                      │
   50 already waiting? ──yes──► 429 (LineFull)     │
            │ no                                   │
            ▼                                      │
   wait in line (FIFO, CONNECT unanswered)         │
            │                                      │
     ┌──────┴────────┐                             │
  slot freed     30 s passed ──► 429 (TimedOut)    │
     │                                             │
     ▼                                             ▼
   accept CONNECT ──► insert into first free slot s ──► send to worker s % 10
                                                          │
   client leaves (worker sees the connection end) ◄───────┘
     │
     ▼
   remove from slot ──► drop permit ──► next client in line gets in
```

## 3. How a request is handled

```
client                 worker w (owns this connection)                 Connections
  │  [len][payload] ─► stream future: read 1 byte len, read len bytes
  │                    ── worker wakes with Event::Frame ──
  │                    handle_frame:
  │                      Request::decode(payload)
  │                        ├─ None ──► reply [00] (error frame)
  │                        └─ Some(req)
  │                             handle_request(req)  ── borrow_mut ──►  slot (id-checked)
  │                               GetFen  → Fen(slot.fen)                read/update fen
  │                               SetFen  → store fen, Fen(fen)
  │                               Select  → Select(square)  (echo for now)
  │                               Move    → Move(mv)        (echo for now)
  │                      Response::encode() into the stream's reply buffer
  │                    new stream future: write reply, then read the next frame
  │  ◄─ [len][payload]
```

### Wire frame

```
[ len: u8 ][ payload: len bytes ]      both directions

len  1 → GetFen        (request)
len  8 → Select(u64)   (request and response)
len 17 → Move          from u64 + to u64 + role u8, big-endian
len 32 → SetFen / Fen  64 squares × 4 bits
len  0 → error frame   (response only: request could not be decoded)
```

## 4. How it is tested

`examples/smoke_client.rs` is a real WebTransport client. It includes the
server's own `src/Payload` code (`#[path]`), so its **request dispatcher**
builds requests with `Request::encode` and reads replies with
`Response::decode`, the exact inverse of what the server does.

```
Request ──encode──► payload ──frame [len]──► stream ──► server
Response ◄─decode── payload ◄──read [len]─── stream ◄── server
                      │
                      └── logs: request, raw bytes both ways (hex), round trip, decoded reply
```

| Phase | What happens | Pass condition |
| --- | --- | --- |
| 1 | 100 clients connect; each sends GetFen, SetFen, GetFen, Select, Move, and a bad payload | Every reply equals the expected `Response`; the 2nd GetFen returns the FEN set by SetFen (state kept in the slot); the bad payload gets a 0-byte error frame |
| 2 | All 100 wait at a barrier, then send one Move at the same instant | All 100 correct; prints p50 / max round trip and time until the last reply |
| 3 | 50 more clients connect while all 100 slots are full | After 500 ms, 0 of them admitted |
| 4 | Client 151 connects | Refused with 429 at once (line full) |
| 5 | The first 100 clients leave | All 50 waiters get in, each served GetFen, then leave |

Run it (server first, then pass the `cert sha-256` from its log):

```bash
cargo run --release
cargo run --release --example smoke_client -- <cert-sha256>
```

Measured on localhost, release build, 3 runs, all passed:

| Measurement | Result (range over 3 runs) |
| --- | --- |
| Burst: 100 Moves at the same instant, all replies | 3.4 – 5.8 ms (round trip p50 1.8 – 3.5 ms) |
| Waiters admitted while all slots are full | 0 of 50 |
| Client 151 refused | 2.4 – 6.2 ms, with 429 |
| 50 waiters served after 100 slots freed | 12 – 22 ms |
| Requests per worker (one run) | 63 – 67 each, all 10 workers busy |
| Threads used by the server | `interface`: all 1306 lines of client work; `main`: 2 lines (start, finish) |

Limits of the test: one machine (no real network delay), no browser client,
the 30 s timeout is not exercised, and Select/Move are still echoes until the
chess engine is wired in.
