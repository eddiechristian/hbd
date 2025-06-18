use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap,StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use mysql::prelude::*;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use std::future::Future;
use std::pin::Pin;
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: Option<u32>,
    pub name: String,
    pub email: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct UserQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeartbeatDevice {
    pub id: Option<u32>,
    pub mac_address: Option<String>,
    pub global_ip_address: Option<String>, 
    pub local_ip_address: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_heartbeat: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatQuery {
    #[serde(rename = "ID")]
    pub id: u32,
    #[serde(rename = "MAC")]
    pub mac: String,
    #[serde(rename = "IP")]
    pub ip: String,
    #[serde(rename = "LP")]
    pub long_poll: Option<String>,
    pub timestamp: Option<u64>,
    pub pip: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StoredProcRequest {
    pub mac_address: String,
    pub private_ip_address: String,
    pub public_ip_address: String,
    pub camera_number: Option<i32>,
    pub zone_number: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct StoredProcResponse {
    pub status: String,
    pub method: String,
    pub message: String,
    pub previous_private_ip: Option<String>,
    pub device: Option<DeviceInfo>,
}

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub id: u32,
    pub mac_address: String,
    pub local_ip_address: Option<String>,
    pub global_ip_address: Option<String>,
    pub last_heartbeat: Option<String>,
    pub camera_number: Option<i32>,
    pub zone_number: Option<i32>,
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: mysql::Pool,
    pub cache: crate::cache::HeartbeatCache<'static>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let config = crate::config::Config::load().unwrap_or_default();
        
        // Create connection pool using the enhanced config
        let db_pool = config.create_connection_pool()
            .context("Failed to create database connection pool")?;

        // Initialize the cache
        let cache = crate::cache::HeartbeatCache::new();

        log::info!("Application state initialized with connection pool and cache");

        Ok(AppState { 
            db_pool,
            cache,
        })
    }

    /// Get a connection from the pool
    /// This is much more efficient than creating new connections
    pub fn get_connection(&self) -> anyhow::Result<mysql::PooledConn> {
        self.db_pool.get_conn()
            .context("Failed to get connection from pool")
    }
}

// API Handlers

/// Health check endpoint
pub async fn health() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "mysql_connection_demo"
    })))
}

/// Get database information
pub async fn get_db_info(
    headers: HeaderMap, 
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, StatusCode> {
    log::info!("eddie: headers{:?}", headers);
    match state.get_connection() {
        Ok(mut conn) => {
            let version: Vec<String> = conn.query("SELECT VERSION()")
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let databases: Vec<String> = conn.query("SHOW DATABASES")
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            Ok(Json(serde_json::json!({
                "mysql_version": version.first().unwrap_or(&"Unknown".to_string()),
                "databases": databases,
                "connection_status": "connected"
            })))
        },
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}


/// Handle device heartbeat with mission-critical write-through caching
pub async fn handle_heartbeat(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<HeartbeatQuery>
) -> Result<Json<serde_json::Value>, StatusCode> {
    log::info!("eddie: headers{:?}", headers);
    
    // Use the new cache-enabled heartbeat handler
    crate::app_with_mysql_and_cache::handle_heartbeat_with_cache(
        state.clone(),
        params,
        &state.cache,
        false
    ).await
}

pub async fn handle_heartbeat_uninitialized(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<HeartbeatQuery>
) -> Result<Json<serde_json::Value>, StatusCode> {
    log::info!("eddie: headers{:?}", headers);
    
    // Use the new cache-enabled heartbeat handler
    crate::app_with_mysql_and_cache::handle_heartbeat_with_cache(
        state.clone(),
        params,
        &state.cache,
        true
    ).await
}

/// Direct stored procedure endpoint for testing
pub async fn call_stored_procedure(
    State(state): State<AppState>,
    Json(payload): Json<StoredProcRequest>
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut conn = state.get_connection().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // First ensure device exists (the stored procedure requires existing device)
    let device_exists: Vec<u32> = conn.exec(
        "SELECT id FROM devices WHERE mac_address = UPPER(?)",
        (&payload.mac_address,)
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if device_exists.is_empty() {
        // Insert new device first
        conn.exec_drop(
            "INSERT INTO devices (mac_address, local_ip_address, global_ip_address, last_heartbeat, camera_number, zone_number) VALUES (UPPER(?), ?, ?, NOW(), ?, ?)",
            (&payload.mac_address, &payload.private_ip_address, &payload.public_ip_address, payload.camera_number.unwrap_or(1), payload.zone_number.unwrap_or(1))
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    // Call the stored procedure
    let result: Result<Vec<(u32, String, Option<String>, String)>, mysql::Error> = conn.exec(
        "CALL set_device_last_heartbeat(?, ?, ?, @msg, @prev_ip)",
        (&payload.mac_address, &payload.private_ip_address, &payload.public_ip_address)
    );
    
    match result {
        Ok(rows) => {
            // Get the output parameters
            let output_params: Vec<(Option<String>, Option<String>)> = conn.exec(
                "SELECT @msg as message, @prev_ip as previous_ip",
                ()
            ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            let (message, previous_ip) = output_params.first()
                .map(|(msg, prev)| (msg.clone().unwrap_or("OK".to_string()), prev.clone()))
                .unwrap_or(("OK".to_string(), None));
            
            // Get the procedure result
            let (device_id, proc_message, prev_ip_result, timestamp) = rows.first()
                .map(|(id, msg, prev, ts)| (*id, msg.clone(), prev.clone(), ts.clone()))
                .unwrap_or((0, "Unknown".to_string(), None, "Unknown".to_string()));
            
            // Get the updated device info
            let device_info: Vec<(u32, String, Option<String>, Option<String>, Option<String>, String, String, Option<i32>, Option<i32>)> = conn.exec(
                "SELECT id, mac_address, local_ip_address, global_ip_address, last_heartbeat, created_at, last_modified, camera_number, zone_number 
                 FROM devices WHERE mac_address = UPPER(?)",
                (&payload.mac_address,)
            ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            if let Some((id, mac_addr, local_ip, global_ip, last_hb, created, modified, camera, zone)) = device_info.first() {
                Ok(Json(serde_json::json!({
                    "status": "success",
                    "method": "stored_procedure",
                    "message": message,
                    "previous_private_ip": previous_ip,
                    "procedure_result": {
                        "device_id": device_id,
                        "message": proc_message,
                        "previous_ip_from_procedure": prev_ip_result,
                        "timestamp": timestamp
                    },
                    "device": {
                        "id": id,
                        "mac_address": mac_addr,
                        "local_ip_address": local_ip,
                        "global_ip_address": global_ip,
                        "last_heartbeat": last_hb,
                        "created_at": created,
                        "last_modified": modified,
                        "camera_number": camera,
                        "zone_number": zone
                    }
                })))
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Err(e) => {
            eprintln!("Stored procedure error: {}", e);
            Ok(Json(serde_json::json!({
                "status": "error",
                "method": "stored_procedure",
                "error": e.to_string()
            })))
        }
    }
}


/// Create the Axum router with all routes
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/db-info", get(get_db_info))
        .route("/hbd", get(handle_heartbeat))
        .route("/hbd/uninitialized", get(handle_heartbeat_uninitialized))
        // .route("/api/heartbeat/procedure", post(call_stored_procedure))
        .layer(CorsLayer::permissive())
        .with_state(state)
}