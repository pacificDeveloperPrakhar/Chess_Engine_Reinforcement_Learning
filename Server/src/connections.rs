use std::net::SocketAddr;
use wtransport::{Connection, VarInt};

// ---------------------------------------------------------------------------
// Connection store: every live connection in one heap-allocated Vec
// ---------------------------------------------------------------------------

/// How many clients can be connected at the same time.
pub const MAX_CONNECTIONS: usize = 100;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";

/// Everything the server keeps about one connected client.
pub struct Client {
    /// Unique per session, so a finished session can't touch a reused slot.
    pub id: u64,
    pub conn: Connection,
    pub remote: SocketAddr,
    pub fen: String,
}

/// Fixed-size table of slots, allocated once on the heap at startup.
/// Lives in `Rc<RefCell<..>>`: everything runs on one thread, so no lock is needed.
pub struct Connections {
    slots: Vec<Option<Client>>,
}

impl Connections {
    pub fn new() -> Self {
        Self { slots: (0..MAX_CONNECTIONS).map(|_| None).collect() }
    }

    pub fn live(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Puts the client in the first free slot and returns its index.
    pub fn insert(&mut self, client: Client) -> Option<usize> {
        let slot = self.slots.iter().position(|s| s.is_none())?;
        self.slots[slot] = Some(client);
        Some(slot)
    }

    pub fn get_mut(&mut self, slot: usize, id: u64) -> Option<&mut Client> {
        self.slots.get_mut(slot)?.as_mut().filter(|c| c.id == id)
    }

    pub fn remove(&mut self, slot: usize, id: u64) -> Option<Client> {
        let entry = self.slots.get_mut(slot)?;
        if entry.as_ref()?.id != id {
            return None;
        }
        entry.take()
    }

    pub fn close_all(&self) {
        for client in self.slots.iter().flatten() {
            client.conn.close(VarInt::from_u32(0), b"server shutting down");
        }
    }
}
