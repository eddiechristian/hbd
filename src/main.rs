use mysql::*;
use mysql::prelude::*;
use anyhow::Result;
use dotenv::dotenv;
use log::{info, warn, error, debug};
use syslog::{Facility, Formatter3164, unix};
use colored::*;
use std::io::{self, Write};
use tokio::net::TcpListener;
use clap::{Arg, Command};

mod server;
mod config;
mod app_with_mysql_and_cache;

// Custom syslog writer
struct SyslogWriter {
    writer: syslog::Logger<syslog::LoggerBackend, syslog::Formatter3164>,
}

impl SyslogWriter {
    fn new() -> Result<Self> {
        let formatter = Formatter3164 {
            facility: Facility::LOG_USER,
            hostname: None,
            process: "mysql_rust_demo".into(),
            pid: std::process::id(),
        };
        
        match unix(formatter) {
            Ok(writer) => Ok(SyslogWriter { writer }),
            Err(e) => Err(anyhow::Error::msg(format!("Syslog initialization failed: {}", e))),
        }
    }
    
    fn log(&mut self, level: &str, message: &str) {
        let log_msg = format!("{}", message);
        match level {
            "ERROR" => { let _ = self.writer.err(log_msg); },
            "WARN" => { let _ = self.writer.warning(log_msg); },
            "INFO" => { let _ = self.writer.info(log_msg); },
            "DEBUG" => { let _ = self.writer.debug(log_msg); },
            _ => { let _ = self.writer.info(log_msg); },
        }
    }
}

// Function to print colored messages to console
fn print_colored(level: &str, message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let colored_level = match level {
        "INFO" => "INFO".bright_blue(),
        "DEBUG" => "DEBUG".bright_black(),
        "WARN" => "WARN".bright_yellow(),
        "ERROR" => "ERROR".bright_red(),
        _ => "INFO".bright_blue(),
    };
    
    println!("{} [{}] mysql_connection_demo - {}", 
             timestamp.to_string().bright_black(),
             colored_level,
             message);
    io::stdout().flush().unwrap_or(());
}

// Macro to log to log4rs (file), syslog, and colored console
macro_rules! log_both {
    ($syslog:expr, $level:expr, $($arg:tt)*) => {
        let message = format!($($arg)*);
        
        // Print colored console output
        print_colored(&$level.to_uppercase(), &message);
        
        // Log to log4rs (this will go to file only now)
        match $level {
            "info" => info!($($arg)*),
            "error" => error!($($arg)*),
            "warn" => warn!($($arg)*),
            "debug" => debug!($($arg)*),
            _ => info!($($arg)*),
        }
        
        // Log to syslog
        if let Some(ref mut logger) = $syslog {
            logger.log(&$level.to_uppercase(), &message);
        }
    };
}


#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("not sure what this should ber")
        .version("1.0")
        .about("heartbeat daemon")
        .arg(Arg::new("benchmark")
            .long("benchmark")
            .help("Run in benchmark mode")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("devices-per-pod")
            .long("devices-per-pod")
            .help("Number of devices to simulate per pod")
            .value_name("COUNT")
            .default_value("1000"))
        .arg(Arg::new("pod-index")
            .long("pod-index")
            .help("Pod index for device ID offset calculation")
            .value_name("INDEX")
            .default_value("0"))
        .arg(Arg::new("server")
            .long("server")
            .help("Run as web server")
            .action(clap::ArgAction::SetTrue))
        .get_matches();

    // Initialize log4rs from configuration file
    log4rs::init_file("log4rs.yaml", Default::default())
        .unwrap_or_else(|e| {
            eprintln!("Failed to initialize log4rs: {}. Falling back to console logging.", e);
            // Fallback to simple console logging if config file fails
            env_logger::init();
        });
        
    // Initialize syslog
    let mut syslog_writer = match SyslogWriter::new() {
        Ok(writer) => {
            print_colored("INFO", "Syslog initialized successfully");
            info!("Syslog initialized successfully");
            Some(writer)
        },
        Err(e) => {
            print_colored("WARN", &format!("Failed to initialize syslog: {}. Continuing without syslog.", e));
            warn!("Failed to initialize syslog: {}. Continuing without syslog.", e);
            None
        }
    };
    
    // Load environment variables from .env file
    // dotenv().ok();


    // Load configuration using the new config module
    let config = match config::Config::load() {
        Ok(cfg) => {
            log_both!(syslog_writer, "info", "Configuration loaded successfully");
            log_both!(syslog_writer, "debug", "Environment: {}", cfg.app.environment);
            log_both!(syslog_writer, "debug", "Debug mode: {}", cfg.app.debug);
            cfg
        },
        Err(e) => {
            log_both!(syslog_writer, "error", "Failed to load configuration: {}", e);
            log_both!(syslog_writer, "info", "Using default configuration");
            config::Config::default()
        }
    };

    // Start HTTP server
    log_both!(syslog_writer, "info", "🚀 Starting HTTP server...");
    start_http_server(syslog_writer, &config).await?;
}

async fn start_http_server(mut syslog_writer: Option<SyslogWriter>, config: &config::Config) -> Result<()> {
    // Create application state
    let state = server::AppState::new()?;
    
    // Create the router
    let app = server::create_router(state);
    
    // Use config for server settings
    let bind_address = config.server_address();
    
    log_both!(syslog_writer, "info", "HTTP server binding to {}", bind_address);
    
    // Create TcpListener
    let listener = TcpListener::bind(&bind_address).await
        .map_err(|e| anyhow::Error::msg(format!("Failed to bind to {}: {}", bind_address, e)))?;
    
    log_both!(syslog_writer, "info", "✅ HTTP server started successfully!");
    log_both!(syslog_writer, "info", "📡 Server listening on http://{}", bind_address);
    log_both!(syslog_writer, "info", "Available endpoints:");
    log_both!(syslog_writer, "info", "  GET  /health           - Health check");
    log_both!(syslog_writer, "info", "  GET  /api/db-info      - Database information");
    log_both!(syslog_writer, "info", "  GET  /api/users        - List all users (supports ?limit=N&offset=N)");
    log_both!(syslog_writer, "info", "  POST /api/users        - Create new user");
    log_both!(syslog_writer, "info", "  GET  /api/users/:id    - Get user by ID");
    log_both!(syslog_writer, "info", "  GET  /hbd              - Device heartbeat endpoint (supports ?ID=123&MAC=000&IP=192.168.1.1&ts=1749862684)");
    log_both!(syslog_writer, "info", "  GET  /demo/twisted-style - Demo of Twisted-style async callbacks");
    log_both!(syslog_writer, "info", "  GET  /api/ignite/info  - Apache Ignite cluster information");
    log_both!(syslog_writer, "info", "  GET  /api/ignite/sql   - Execute SQL on Ignite (supports ?sql=query)");
    log_both!(syslog_writer, "info", "  GET  /api/users/:id/hybrid - Hybrid MySQL+Ignite user lookup");
    log_both!(syslog_writer, "info", "  GET  /demo/performance - Real-time MySQL vs Ignite performance comparison");
    log_both!(syslog_writer, "info", "  GET  /api/benchmark/info - Benchmark configuration info");
    log_both!(syslog_writer, "info", "  GET  /api/benchmark/run - High-load benchmark (200+ RPS simulation)");
    log_both!(syslog_writer, "info", "🔥 Press Ctrl+C to stop the server");
    
    // Start the server
    axum::serve(listener, app).await
        .map_err(|e| anyhow::Error::msg(format!("Server error: {}", e)))?;
    
    Ok(())
}