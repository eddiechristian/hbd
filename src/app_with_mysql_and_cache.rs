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

struct AuthorizedResult{
    authorized: bool,
    squelched: bool,
}

fn get_pip()-> String{
    return "127.1.1.0".to_string();
}

fn get_status_code() -> Result<Json<serde_json::Value>,StatusCode>{
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
/// is mac in cache or db
fn get_authorized(cache: &HeartbeatCache, mac: &str) -> AuthorizedResult{
    match cache.get_device(mac){
        None => {
            let mut authorized:bool = false;
            let mut squelched:bool = false;
            //call db to get auth and squelched.
            AuthorizedResult { authorized: authorized, squelched: squelched }
        },
        Some(_)=>  AuthorizedResult { authorized: true, squelched: false }
    }

}

fn get_last_heartbeat_write(cache: &HeartbeatCache ,mac: &str) -> Option<DateTime<Utc>>{
    match cache.get_device(mac){
        None => None,
        Some(cached_device)=> cached_device.last_heartbeat_write
    }

} 

/// Handle heartbeat with MySQL and cache integration
/// This function can be called from handle_heartbeat in server.rs
pub async fn handle_heartbeat_with_cache(
    state: AppState,
    params: HeartbeatQuery,
    cache: &HeartbeatCache<'_>,
    uninitialized: bool,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let device_id = params.id;
    let mac_address = params.mac.clone();
    let ip_address = params.ip.clone();
    let timestamp = params.timestamp;
    let LP = params.long_poll;
    let pip = get_pip();
    let authorized= get_authorized(cache, &mac_address);
    let last_heartbeat_write =get_last_heartbeat_write(cache, &mac_address);
    log::info!("Processing heartbeat for device ID: {}, MAC: {:?}, IP: {:?}", 
    device_id, mac_address, ip_address);

    //if not authorized
      // remove from hb_waiting cache, cache
      // redirect to 

    //if authorized but squelched 
    // redirect to
    

    //if device exists in cache
      // if eiher ip changes, write_to_db(set_device_last_heartbeat)
      // if pip changes notify frontend
      // if uninitialized write to db

    // if uninitialized
      // call sp: set_ready_device

    // if now - cache entry_last_db_write > (max_hb_staleness - hb_staleness_period)  
      // update db(sp : update_last_hb) if cache_entry_last_heartbeat > entry_last_db_write

    // update cache either way
    let device_update = CachedDevice{
        id: device_id,
        mac_address: mac_address,
        global_ip_address: pip,
        local_ip_address: ip_address,
        last_heartbeat: Utc::now(),
        last_heartbeat_write: last_heartbeat_write,
    };
    cache.update_device(device_update);
   
    get_status_code()
      
}
