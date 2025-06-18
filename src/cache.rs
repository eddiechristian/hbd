use lockfreehashmap::LockFreeHashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use crossbeam_utils::atomic::AtomicCell;

/// Simple in-memory cache for heartbeat data
#[derive(Debug, Clone)]
pub struct HeartbeatCache<'a> {
    pub devices: Arc<LockFreeHashMap<'a, u32, CachedDevice>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CachedDevice {
    pub id: u32,
    pub mac_address: String,
    pub global_ip_address: String,
    pub local_ip_address: String,
    pub last_heartbeat: DateTime<Utc>,
    pub cache_timestamp: DateTime<Utc>,
}

impl<'a> HeartbeatCache<'a> {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(LockFreeHashMap::new()),
        }
    }
}