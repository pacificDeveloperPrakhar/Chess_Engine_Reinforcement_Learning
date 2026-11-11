use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use tokio::task::JoinHandle;

pub struct Manager
{
    pub id: u64,
    pub ip: String,
    pub port: u16,
}

pub struct ConsistentHashRing
{
    // position on the ring -> manager id (only managers are placed on the ring)
    ring: BTreeMap<u64, u64>,
    duplicacy_factor: u64,
    // manager id -> the task running that manager
    managers: &'static Mutex<HashMap<u64, JoinHandle<()>>>,
}

impl ConsistentHashRing
{
    pub fn new(duplicacy_factor: u64) -> Self
    {
        // get_or_init returns &'static, because MANAGERS is a static
        let managers = crate::MANAGERS.get_or_init(|| Mutex::new(HashMap::new()));

        Self {
            ring: BTreeMap::new(),
            duplicacy_factor,
            managers,
        }
    }

    pub fn add_manager(&mut self, manager: Manager)
    {
        for i in 0..self.duplicacy_factor {
            let hash = self.hash(&format!("{}-{}", manager.id, i));
            self.ring.insert(hash, manager.id);
        }
    }

    pub fn remove_manager(&mut self, manager_id: u64)
    {
        self.ring.retain(|_, id| *id != manager_id);
    }

    // this is a simple hashing function that uses the default hasher to hash the key and return a u64 value
    pub fn hash(&self, key: &str) -> u64
    {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_manager_for_connection(&self, connection_id: u64) -> Option<u64>
    {
        let hash = self.hash(&format!("{}-xyz", connection_id));
        // first manager point at or after the connection's hash; wrap to the start if there is none
        self.ring
            .range(hash..)
            .next()
            .or_else(|| self.ring.iter().next())
            .map(|(_, &manager_id)| manager_id)
    }
}
