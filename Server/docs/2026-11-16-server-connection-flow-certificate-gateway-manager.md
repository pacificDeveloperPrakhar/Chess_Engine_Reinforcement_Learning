# Server Connection Flow: Certificate, Gateway and Manager Runtime

**Date:** 2026-11-16  
**Area:** Server  
**Status:** In progress

## Summary
This note follows one client from start to finish. First the server makes its own key pair and a self-signed certificate. Then the browser pins that certificate and connects over WebTransport. The gateway accepts the connection and reads each message from a bidirectional stream. Finally it hands the message to a manager task, which replies on the same stream. Every concept below was checked against official docs or the RFCs, and the corrections sit next to what I first said.

## What I did
1. Generate an ECDSA P-256 key pair (`--generate`) and save it as `private_key.pem` / `public_key.pem`.
2. Build the X.509 certificate by hand from Rust structs that mirror the ASN.1 definitions, encode it to DER, sign it, and save `certificate.pem` (`--gen_cert`).
3. Pin the certificate in the browser (the WebTransport Tester takes a SHA-256 hash of it).
4. Start the gateway on port 8081. Each connection gets an ID, and each bidirectional stream is read as `[length][payload]`.
5. Send the payload to the right manager task (picked by consistent hashing) over an MPSC channel. The manager replies on the same stream.
- **Problem:** I first thought that `send_datagram` / receiving a datagram would reuse the stream I had opened. It doesn't. Datagrams don't use streams at all, so request/reply now goes over the bidirectional stream.
- **Planned (not done yet):** move the `OnceLock` statics to `LazyLock`, and maybe wrap `CONNECTIONS` in a read-write lock instead of a `Mutex`. For now they stay as they are.

## Concepts

### 1. ECDSA P-256 key pair
**In plain words:** The private key is a big random number that only the server knows. The public key is a point on the P-256 curve, worked out from the private key. Anyone can use the public key to check signatures made with the private key, but nobody can work backwards to the private key.  
**Analogy:** A wax seal. Only you own the seal stamp (private key), but everyone knows what your seal looks like (public key), so they can check that a letter really came from you.  
**Minimal example** (`Server/src/Certificates/keys.rs`, lines 13–15):
```rust
let signing_key = SigningKey::random(&mut rng);          // private key
let verifying_key = *signing_key.verifying_key();        // public key
```

### 2. ASN.1 and DER (the certificate struct)
**In plain words:** ASN.1 is a language that *describes* the shape of data (which fields, in what order, what type). DER is a strict rule for turning that shape into exact bytes. There is only one valid DER encoding, so a signature over the bytes always checks out the same way. X.509 certificates are DER-encoded, and the signature covers the DER bytes of `tbsCertificate` (the "to be signed" part).  
**Analogy:** ASN.1 is the blank form with labelled boxes. DER is the rule "fill in every box in this order, in this exact handwriting", so two copies of the same data come out byte-for-byte identical.  
**Minimal example** (`Server/src/Certificates/certificate_asn.rs`, lines 165–171, the outer certificate):
```rust
#[derive(Debug,Clone,Eq,PartialEq,Sequence)]
pub struct Certificate
{
    pub tbs_certificate:tbsCertificate,
    pub signature_algorithm:AlgorithmIdentifier,
    pub signature:BitString
}
```
The inner `tbsCertificate` (version, serial, issuer, validity, subject, public key, extensions) is at lines 113–135. Signing happens in `Server/src/main.rs`, line 153:
```rust
let signature: Signature = signing_key.sign(&tbs.to_der().unwrap());
```

### 3. PKCS#8 and SubjectPublicKeyInfo
**In plain words:** PKCS#8 is a **standard format** (RFC 5208, updated by RFC 5958) for storing a **private key**. It is not an algorithm. The **public key** is stored in a different standard format called **SubjectPublicKeyInfo** (from RFC 5280). Both are DER bytes, and a `.pem` file is just those bytes in base64 between `-----BEGIN ...-----` lines.  
**Analogy:** Two different standard envelope sizes: one for the secret key and one for the public key. The envelope doesn't change the key. It only makes sure every tool knows how to open it.  
**Minimal example** (`Server/src/main.rs`, line 78 imports; lines 96–97 decode):
```rust
use p256::pkcs8::{DecodePrivateKey, DecodePublicKey};
let signing_key  = SigningKey::from_pkcs8_pem(&priv_key).unwrap();        // PKCS#8
let verifying_key = VerifyingKey::from_public_key_pem(&pub_key).unwrap(); // SubjectPublicKeyInfo
```

### 4. Certificate pinning with `serverCertificateHashes`
**In plain words:** Normally a browser trusts a certificate because a known authority signed it. With pinning, you tell the browser in advance: "trust the server whose certificate has *this* SHA-256 hash". The browser keeps only the **hash**, not the certificate or the public key. The public key is already inside the certificate. The certificate must be X.509v3, valid for **less than 2 weeks**, and use an ECDSA P-256 key (RSA keys are not allowed).  
**Analogy:** You keep a fingerprint of the server's ID card. When the server shows up, you take its fingerprint again and compare.  
**Minimal example** (`Client/App.js`, lines 143 and 165–166):
```js
const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", der));
new WebTransport(url, { serverCertificateHashes: [{ algorithm: "sha-256", value: digest }] });
```

### 5. TLS 1.3 handshake (inside QUIC)
**In plain words:** The order is: ClientHello → ServerHello → EncryptedExtensions → server **Certificate** → server **CertificateVerify** → Finished. The certificate was signed once, when we created it. **CertificateVerify** is a *fresh* signature over this handshake, made with the server's private key. The browser does two checks:
1. Hash the certificate it just received and compare it with the pin.
2. Check the CertificateVerify signature using the public key **from that same certificate**.

**Analogy:** The pin is a saved photo of the ID card. CertificateVerify is the person signing their name in front of you right now, so you know they hold the card and didn't just copy it.  
**Minimal example** (server side; wtransport/TLS does the handshake for us, `Server/src/main.rs`, line 201):
```rust
let identity = Identity::load_pemfiles("certificate.pem", "private_key.pem").await.unwrap();
```

### 6. `OnceLock` and `LazyLock` statics
**In plain words:** Both let a `static` be filled in at runtime instead of at compile time. `LazyLock` holds the setup code itself and runs it **on first access**. `LazyLock::force` runs that setup right now. `OnceLock` starts empty, and you fill it with `get_or_init` wherever you first use it.  
**Analogy:** `LazyLock` is a coffee machine that brews the first time someone presses the button. `force` is pressing the button yourself in the morning so it's ready before anyone else arrives.  
**Minimal example** (`Server/src/main.rs`, lines 36–38 and 48):
```rust
pub static GATEWAY_RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
});
LazyLock::force(&GATEWAY_RT); // build it now, on the main thread
```

### 7. `Mutex` vs `RwLock`
**In plain words:** A `Mutex` lets only one user in at a time. An `RwLock` lets **many readers at once, or one writer**. While someone writes, everyone else (readers and writers) waits. Who goes first when both are waiting depends on which `RwLock`:
- `std::sync::RwLock`: **no promise**. The priority depends on the operating system.
- `tokio::sync::RwLock`: **fair**. Waiters are served first come, first served, so a waiting writer isn't skipped by new readers.

**Analogy:** A library notice board. Many people can read it at the same time, but when someone is changing the notice, everyone steps back until they're done.  
**Minimal example:**
```rust
let r1 = lock.read().unwrap();  // many readers OK
let r2 = lock.read().unwrap();
// lock.write() would wait until r1 and r2 are dropped
```

### 8. Atomic IDs and `Ordering::Relaxed`
**In plain words:** `fetch_add` on an `AtomicU64` adds 1 in a single step that can't be interrupted, so every connection gets a unique ID **even with many threads**. `Relaxed` means "no ordering promise relative to *other* memory reads and writes". That applies to the CPU as well as the compiler. A plain ID counter doesn't need that promise, so `Relaxed` is the right choice. Connection IDs start at 2^63, so their top bit is 1. Manager IDs start at 0.  
**Analogy:** A ticket machine at a bank. Everyone gets a different number, even if two people press at once. `Relaxed` just means the machine doesn't also promise anything about what else happened in the bank at that moment.  
**Minimal example** (`Server/src/main.rs`, lines 40 and 222):
```rust
static NEXT_ID_CONNECTION: AtomicU64 = AtomicU64::new(9223372036854775808);
let id = NEXT_ID_CONNECTION.fetch_add(1, Ordering::Relaxed);
```

### 9. Cloning a `wtransport::Connection`
**In plain words:** Cloning a `Connection` doesn't make a new connection. It gives another **handle** to the same one. One handle reads requests, and the other is stored in `CONNECTIONS`.  
**Analogy:** Two remote controls for the same TV.  
**Minimal example** (`Server/src/main.rs`, lines 224–225 and 256):
```rust
let listener_conn = connection.clone(); // used to accept streams
let stored_conn = connection.clone();   // goes into CONNECTIONS
CONNECTIONS...insert(id, (connection_handle, stored_conn));
```

### 10. Bidirectional streams vs datagrams
**In plain words:** A **bidirectional stream** is a two-way, ordered, reliable pipe inside the connection. The browser opens one with `createBidirectionalStream()`, and the server accepts it with `accept_bi()`. A **datagram** is a single, unreliable message sent on the connection itself. It doesn't belong to any stream, and it doesn't open one. To tie a reply to a request, write it on the stream the request came on.  
**Analogy:** A stream is a phone call: ordered, and both sides talk on the same line. A datagram is a postcard: it may arrive or not, and it has nothing to do with the call.  
**Minimal example** (`Server/src/main.rs`, line 228; `Client/App.js`, line 267):
```rust
let (send, mut recv) = listener_conn.accept_bi().await?;
```
```js
const stream = await conn.wt.createBidirectionalStream();
```

### 11. Length-prefix framing
**In plain words:** A QUIC stream is just an ordered run of bytes with no "message start/end" markers. So we put the message length first: **1 byte of length**, then that many payload bytes. One byte means a message can be at most **255 bytes**.  
**Analogy:** Telling someone "the next 17 words are the address" before you read it out.  
**Minimal example** (`Server/src/main.rs`, lines 233–238):
```rust
let mut len = [0u8; 1];
recv.read_exact(&mut len).await?;
let mut payload = vec![0u8; len[0] as usize];
recv.read_exact(&mut payload).await?;
```

### 12. The route byte
**In plain words:** One byte says what kind of message this is and which handler it should go to. The **top bit (bit 7, the MSB)** says request or response. In the code, **0 = request** and **1 = response** (see the correction in Verification). The lower 7 bits pick the handler.  
**Analogy:** The first digit of an extension number says "internal call or external call". The rest picks which desk rings.  
**Minimal example:**
```rust
let route = buff[buff.len() - 1];            // Utility/mod.rs, line 18: last byte of a request
route if route < (1 << 7) => { /* request */ } // ManagerRuntime.rs, line 32
let route: u8 = route | (1 << 7);            // Payload/mod.rs, line 62: mark a response
```

### 13. `Payload`, `request_handle` and the MPSC channel
**In plain words:** `request_handle` gets the connection ID, the payload bytes (the "buffer") and the stream's `SendStream`. In wtransport, `SendStream` is a **struct**, not a trait. It implements tokio's `AsyncWrite`, `RecvStream` implements `AsyncRead`, and the two share no read/write trait. The handler splits off the route byte and builds a `Payload`. It then asks the consistent hash which manager owns this connection, gets that manager's channel **sender**, and sends the `Payload` through it.  
**Analogy:** A post office counter. The clerk puts your letter and a reply envelope (the `SendStream`) into one packet. They look up which sorting room handles your address and drop the packet into that room's chute (the channel).  
**Minimal example** (`Server/src/Payload/mod.rs`, lines 4–10; `Server/src/Utility/mod.rs`, lines 20 and 30–36):
```rust
pub struct Payload { pub id:u64, pub data:Vec<u8>, pub route:u8, pub send:Option<wtransport::SendStream> }

let manager_id = CONSISTENT_HASH...get_manager_for_connection(conn_id).unwrap();
let tx = MANAGERS...[&manager_id].1.clone();
tx.send(pd).await?;
```

### 14. Manager runtime
**In plain words:** The manager runtime is a separate multi-thread Tokio runtime with 4 worker threads. Each manager is one task looping on its **MPSC receiver** (`rx`). This is the channel's receiving end, not a QUIC `RecvStream`. For each request it does the work and writes the reply on the **same** `SendStream`, so the reply goes back on the stream the client opened.  
**Analogy:** A sorting room worker who takes packets from the chute one by one and posts each reply in the envelope that came with it.  
**Minimal example** (`Server/src/ManagerRuntime.rs`, lines 27–37):
```rust
let (tx, mut rx) = mpsc::channel::<Payload>(32);
rt.spawn(async move {
    while let Some(pd) = rx.recv().await {
        if let Some(mut send) = pd.send { send_resp(&mut send, b"...").await; }
    }
});
```

## How this fits the project
| Concept | Where in the code | Notes |
|---|---|---|
| Key pair | `Server/src/Certificates/keys.rs` → `fn generate_keypair` (10–17) | The private key comes from a 64-bit seed in `CERT_RAND`. Anyone who knows the seed can rebuild the key. That's fine for local testing, but use `OsRng` before this goes anywhere real. |
| Save keys (PKCS#8 / SPKI PEM) | `keys.rs` → `fn save_keypair_to_files` (19–26) | |
| Certificate structs (ASN.1) | `Server/src/Certificates/certificate_asn.rs` → `Certificate` (165–171), `tbsCertificate` (113–135) | |
| Build + sign certificate | `Server/src/main.rs` → `fn create_certificate` (91–172) | Validity is now → now + 14 days (line 118). MDN says *less than* 2 weeks, so 13 days is safer. |
| Certificate endpoint | `main.rs` → `fn certificate_endpoint` (174–189), UDP port 8082 | Sends `certificate.pem` only. In the Tester you paste the PEM in by hand. |
| Pin + connect | `Client/App.js` → `computePin` (139–151), `openConnection` (165–166) | |
| Statics | `main.rs` lines 21, 28–30 (`OnceLock`), 36–38 (`LazyLock`), 48 (`force`) | Plan: move the `OnceLock` ones to `LazyLock`. |
| Gateway setup | `main.rs` → `fn gateway_init` (191–209), port 8081 | `public_key.pem` is read (195) only to check it exists. The identity uses the certificate + private key (201). |
| Session accept | `main.rs` lines 212–221 | The WebTransport session request is accepted before any stream is opened. |
| Connection ID | `main.rs` line 222 | |
| Connection clones | `main.rs` lines 224–225, stored at 256 | |
| Stream accept + framing | `main.rs` lines 226–244 | Each stream is handled in its own task (232). |
| Route byte | `Utility/mod.rs` line 18; `ManagerRuntime.rs` line 32; `Payload/mod.rs` line 62 | Requests carry the route as the **last** byte, but `send_encode_resp` puts it **first** (line 72). In `sendMove` (`App.js` 292) the `role` byte ends up being read as the route. |
| `Payload` struct | `Server/src/Payload/mod.rs` (4–10) | |
| `request_handle` | `Server/src/Utility/mod.rs` (10–38) | |
| Manager runtime | `Server/src/ManagerRuntime.rs` → `manager_init` (15–23), `create_manager` (25–60), `add_manager` (93–100) | Right now it sends a fixed default reply. Decoding the request is still commented out (42–52). |
| Reply on same stream | `ManagerRuntime.rs` lines 35–37 → `Payload/mod.rs` `send_resp` (46–51) | `send_resp` writes a 1-byte length + body, with no route byte. |

## Verification
| Claim | Result | Source |
|---|---|---|
| Keys are generated with ECDSA on an elliptic curve | ✅ P-256 (secp256r1) | [MDN WebTransport()](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport/WebTransport) |
| The certificate is built by hand from a struct defined in ASN.1 | ✅ The Rust structs mirror the ASN.1 definitions, and the `der` derive macros do the encoding | [RFC 5280](https://datatracker.ietf.org/doc/html/rfc5280) |
| It is converted to raw bytes with DER, the standard practice | ✅ X.509 certificates are DER-encoded, and the signature is over the DER `tbsCertificate` | [ITU-T X.690](https://www.itu.int/rec/T-REC-X.690), [RFC 9881 §3](https://www.rfc-editor.org/rfc/rfc9881.html) |
| Browsers accept ECDSA certificates for WebTransport | ✅ P-256 is required, RSA is not allowed, and validity must be **< 2 weeks** | [MDN WebTransport()](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport/WebTransport) |
| For pinning, the server sends the certificate and the public key | ⚠️ Only the certificate is sent. The public key is already inside it. | `main.rs` 186–187 |
| The browser stores the certificate and public key | ⚠️ It keeps only a SHA-256 hash of the DER certificate | [MDN WebTransport()](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport/WebTransport) |
| Handshake: hello, acknowledgement, the server sends a certificate signed with its private key, and the browser verifies it with the pinned public key | ⚠️ The order is ClientHello → ServerHello → … → Certificate → CertificateVerify → Finished. CertificateVerify is a fresh signature over the handshake, and it is checked with the public key from the certificate *received in the handshake*. The pin is a hash match. | [RFC 8446 §2, §4.4.3](https://www.rfc-editor.org/rfc/rfc8446) |
| After connecting, the browser opens a bidirectional stream | ✅ The WebTransport session request is accepted first (`main.rs` 213–221) | [wtransport Connection](https://docs.rs/wtransport/latest/wtransport/struct.Connection.html) |
| `LazyLock` initializes on first access, and `force` at line 48 does it now | ✅ | [std LazyLock](https://doc.rust-lang.org/std/sync/struct.LazyLock.html) |
| RwLock: many readers in parallel, and a writer blocks everyone else | ✅ | [std RwLock](https://doc.rust-lang.org:443/std/sync/struct.RwLock.html) |
| RwLock always prefers writers over readers | ❌ for `std` (OS-dependent, no guarantee); ✅ roughly for `tokio` (fair, first come first served) | [std RwLock](https://doc.rust-lang.org:443/std/sync/struct.RwLock.html), [tokio RwLock](https://docs.rs/tokio/*/tokio/sync/struct.RwLock.html) |
| PKCS#8 is an algorithm for arranging the private and public key bytes | ⚠️ It's a standard *format* for the **private** key only. The public key uses SubjectPublicKeyInfo. | [RFC 5958](https://datatracker.ietf.org/doc/html/rfc5958) |
| The gateway reads the private and public key, then serves on 8081 | ✅ | `main.rs` 193–209 |
| Line 222 gives every new connection a new ID | ✅ | `main.rs` 222 |
| `Relaxed` tells the compiler we don't care about order, and it's OK because only one thread increments | ⚠️ It also applies to the CPU. The add is still atomic, so IDs stay unique even with many threads. | [std atomic Ordering](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html) |
| Two clones: one handles requests, one goes in the map | ✅ A clone is another handle to the same connection | [wtransport Connection](https://docs.rs/wtransport/latest/wtransport/struct.Connection.html) |
| Sending a datagram reuses the open stream (or opens a new one) | ❌ Datagrams are sent on the connection and don't use streams | [wtransport Connection](https://docs.rs/wtransport/latest/wtransport/struct.Connection.html) |
| QUIC streams have no message boundaries, so a length comes first | ✅ Streams are ordered byte streams. A 1-byte length caps messages at 255 bytes. | [RFC 9000 §2](https://www.rfc-editor.org/rfc/rfc9000.html) |
| The route is the first byte of the payload | ⚠️ In a request it's the **last** byte (`Utility/mod.rs` 18). Responses put it first (`Payload/mod.rs` 72). | code |
| MSB of route: 1 = request, 0 = response | ❌ The code does the opposite: **0 = request** (`ManagerRuntime.rs` 32, `route < 128`) and **1 = response** (`Payload/mod.rs` 62). My first answer in the call was the right one. | code |
| `request_handle` takes any object implementing a send-stream trait | ❌ It takes `wtransport::SendStream`, a struct. It implements `AsyncWrite`. `RecvStream` implements `AsyncRead`, and they share no read/write trait. | [wtransport stream.rs](https://docs.rs/crate/wtransport/0.7.1/source/src/stream.rs) |
| `request_handle` gets the manager ID, then the manager, and passes it the payload | ✅ More exactly, it gets the manager's channel **sender** and sends the `Payload` through it | `Utility/mod.rs` 20–36 |
| The manager task at line 28 waits on the receive stream | ⚠️ It waits on the **MPSC receiver** (`rx`), not a QUIC `RecvStream` | `ManagerRuntime.rs` 27–29 |
| The reply goes back on the stream the client opened | ✅ | `ManagerRuntime.rs` 35–37 |

## Resources
No resources were listed while explaining. These are the official sources used to check this note. I fetched each one during verification, and all of them worked.

| Resource | Link works | Official/reliable | Covers the topic | Notes |
|---|---|---|---|---|
| [MDN: WebTransport() constructor](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport/WebTransport) | ✅ | ✅ | ✅ | `serverCertificateHashes` and certificate rules |
| [RFC 8446: TLS 1.3](https://www.rfc-editor.org/rfc/rfc8446) | ✅ | ✅ | ✅ | Handshake order and CertificateVerify |
| [RFC 9881](https://www.rfc-editor.org/rfc/rfc9881.html) | ✅ | ✅ | ⚠️ | About ML-DSA certificates. Used only for the line saying the signature is over the DER `TBSCertificate`. RFC 5280 is the main source. |
| [RFC 5958: Asymmetric Key Packages (PKCS#8)](https://datatracker.ietf.org/doc/html/rfc5958) | ✅ | ✅ | ✅ | |
| [RFC 9000: QUIC](https://www.rfc-editor.org/rfc/rfc9000.html) | ✅ | ✅ | ✅ | Streams as ordered byte streams |
| [ITU-T X.690 (DER)](https://www.itu.int/rec/T-REC-X.690) | ✅ | ✅ | ✅ | |
| [std::sync::RwLock](https://doc.rust-lang.org:443/std/sync/struct.RwLock.html) | ✅ | ✅ | ✅ | |
| [tokio::sync::RwLock](https://docs.rs/tokio/*/tokio/sync/struct.RwLock.html) | ✅ | ✅ | ✅ | |
| [std::sync::LazyLock](https://doc.rust-lang.org/std/sync/struct.LazyLock.html) | ✅ | ✅ | ✅ | |
| [std::sync::atomic::Ordering](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html) | ✅ | ✅ | ✅ | |
| [wtransport Connection](https://docs.rs/wtransport/latest/wtransport/struct.Connection.html) | ✅ | ✅ | ⚠️ | Shows the latest version. The project uses `wtransport = "0.6"`. |
| [wtransport stream.rs (0.7.1 source)](https://docs.rs/crate/wtransport/0.7.1/source/src/stream.rs) | ✅ | ✅ | ⚠️ | Version 0.7.1. The project is on 0.6. |

## Open questions / next steps
- Replace the `OnceLock` statics (`CERT`, `CONNECTIONS`, `MANAGERS`, `CONSISTENT_HASH`) with `LazyLock`.
- Consider wrapping the `CONNECTIONS` hash map in a read-write lock instead of a `Mutex` (kept as `Mutex` for now).
