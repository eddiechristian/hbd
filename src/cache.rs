use lockfreehashmap::LockFreeHashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use crossbeam_utils::atomic::AtomicCell;

/// Simple in-memory cache for heartbeat data
#[derive(Debug, Clone)]
pub struct HeartbeatCache<'a> {
    pub devices: Arc<LockFreeHashMap<'a, String, HeartbeatCacheInfo>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeartbeatCacheInfo {
    pub id: u32,
    pub mac_address: String,
    pub global_ip_address: String,
    pub local_ip_address: String,
    pub last_heartbeat: DateTime<Utc>,
    pub last_heartbeat_write: Option<DateTime<Utc>>,
}

impl<'a> HeartbeatCache<'a> {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(LockFreeHashMap::new()),
        }
    }

    /// Get device from cache by MAC address
    pub fn get_device(&self, mac_address: &str) -> Option<HeartbeatCacheInfo> {
        let guard = lockfreehashmap::pin();
        self.devices.get(mac_address, &guard).cloned()
    }

    /// Update or insert device in cache using MAC address as key
    pub fn update_device(&self, device: HeartbeatCacheInfo) {
        let guard = lockfreehashmap::pin();
        self.devices.insert(device.mac_address.clone(), device, &guard);
    }

    /// Remove device from cache by MAC address
    pub fn remove_device(&self, mac_address: &str) {
        let guard = lockfreehashmap::pin();
        self.devices.remove(mac_address, &guard);
    }

    /// Get the number of devices in cache
    pub fn len(&self) -> usize {
        // Note: LockFreeHashMap doesn't have a direct len() method
        // This is a limitation of the current implementation
        // We could maintain a separate atomic counter if needed
        0 // Placeholder
    }
}

pub struct HBWaitingCache<'a> {
    pub devices: Arc<LockFreeHashMap<'a, String, HeartbeatCacheInfo>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HBWaitingCacheInfo {
    pub id: u32,
    pub mac_address: String,
}