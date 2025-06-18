use anyhow::{Result, Context};
use axum::{
    http::StatusCode,
    response::Json,
};
use mysql::prelude::*;
use serde_json::json;
use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::server::{AppState, HeartbeatQuery, HeartbeatDevice};
use crate::cache::{HeartbeatCache, CachedDevice};

pub fn get_pip()-> String{
    return "127.1.1.0".to_string();
}

pub fn get_status_code() -> Result<Json<serde_json::Value>,StatusCode>{
    let mut result:StatusCode  = StatusCode::OK;

    match result{
        StatusCode::OK =>{
            Ok(Json(serde_json::json!({
                "status": "success"
                })))
         },
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
    
}
/// Handle heartbeat with MySQL and cache integration
/// This function can be called from handle_heartbeat in server.rs
pub async fn handle_heartbeat_with_cache(
    state: AppState,
    params: HeartbeatQuery,
    cache: &HeartbeatCache<'_>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let device_id = params.id;
    let mac_address = params.mac.clone();
    let ip_address = params.ip.clone();
    let timestamp = params.timestamp;
    let LP = params.long_poll;
    log::info!("Processing heartbeat for device ID: {}, MAC: {:?}, IP: {:?}", 
    device_id, mac_address, ip_address);

    let device = CachedDevice{
        id: device_id,
        mac_address: mac_address,
        global_ip_address: get_pip(),
        local_ip_address: ip_address,
        last_heartbeat: Utc::now(),
        cache_timestamp: Utc::now(),
    };
    let guard = lockfreehashmap::pin();
    cache.devices.insert(device_id, device, &guard);
   
    get_status_code()
      
}
