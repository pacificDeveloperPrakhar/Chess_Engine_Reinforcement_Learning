use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use tokio::task::JoinHandle;

pub struct Manager {
    pub id: u64,
    pub ip: String,
    pub port: u16,
}

pub struct ConsistentHashRing {
    ring: BTreeMap<u64, u64>,
    duplicacy_factor: u64,
    managers: &'static Mutex<HashMap<u64, JoinHandle<()>>>,
    connections: &'static Mutex<HashMap<u64, JoinHandle<()>>>,
}

impl ConsistentHashRing {
    pub fn new(duplicacy_factor: u64) -> Self {
        // get_or_init returns &'static, because CONNECTIONS is a static
        let connections = crate::CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()));
        // get_or_init returns &'static, because MANAGERS is a static
        let managers = crate::MANAGERS.get_or_init(|| Mutex::new(HashMap::new()));

        Self {
            ring: BTreeMap::new(),
            duplicacy_factor,
            managers,
            connections,
        }
    }
}