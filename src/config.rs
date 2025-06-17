use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::env;
use std::fs;
use anyhow::{Result, Context};

/// Main application configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Database configuration
    pub database: DatabaseConfig,
    /// Server configuration
    pub server: ServerConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Application-specific settings
    pub app: AppConfig,
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database host (default: localhost)
    pub host: String,
    /// Database port (default: 3306)
    pub port: u16,
    /// Database username (default: root)
    pub user: String,
    /// Database password (default: password)
    pub password: String,
    /// Database name (default: mysql)
    pub database: String,
    /// Connection pool settings
    pub pool: PoolConfig,
    /// Initialize DB with canned data.
    pub initialize_canned_data: bool,
}

/// Database connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections in pool (default: 1)
    pub min_connections: u32,
    /// Maximum number of connections in pool (default: 10)
    pub max_connections: u32,
    /// Connection timeout in seconds (default: 30)
    pub connection_timeout: u64,
    /// Idle timeout in seconds (default: 600)
    pub idle_timeout: u64,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address (default: 0.0.0.0)
    pub host: String,
    /// Server port (default: 3000)
    pub port: u16,
    /// Request timeout in seconds (default: 30)
    pub request_timeout: u64,
    /// Enable CORS (default: true)
    pub enable_cors: bool,
    /// Maximum request body size in bytes (default: 1MB)
    pub max_body_size: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (default: info)
    pub level: String,
    /// Enable file logging (default: true)
    pub file_enabled: bool,
    /// Log file path (default: logs/app.log)
    pub file_path: PathBuf,
    /// Enable console logging (default: true)
    pub console_enabled: bool,
    /// Enable syslog (default: true)
    pub syslog_enabled: bool,
    /// Maximum log file size in MB (default: 10)
    pub max_file_size: u64,
    /// Number of log files to keep (default: 5)
    pub max_files: u32,
}

/// Application-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Enable debug mode (default: false)
    pub debug: bool,
    /// Heartbeat device settings
    pub cache: Option<CacheConfig>,
}

/// Heartbeat device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default heartbeat interval in seconds (default: 300)
    pub default_interval: u64,
    /// Maximum allowed heartbeat interval in seconds (default: 3600)
    pub max_interval: u64,
    /// Enable automatic cleanup of old heartbeat records (default: true)
    pub auto_cleanup: bool,
    /// Cleanup interval in hours (default: 24)
    pub cleanup_interval: u64,
    /// Maximum age of heartbeat records in days (default: 30)
    pub max_record_age: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            app: AppConfig::default(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 3306,
            user: "root".to_string(),
            password: "password".to_string(),
            database: "mysql".to_string(),
            pool: PoolConfig::default(),
            initialize_canned_data: false,
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 600,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout: 30,
            enable_cors: true,
            max_body_size: 1024 * 1024, // 1MB
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_enabled: true,
            file_path: PathBuf::from("logs/app.log"),
            console_enabled: true,
            syslog_enabled: true,
            max_file_size: 10,
            max_files: 5,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "hbd".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "development".to_string(),
            debug: false,
            cache: None, 
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_interval: 300,
            max_interval: 3600,
            auto_cleanup: true,
            cleanup_interval: 24,
            max_record_age: 30,
        }
    }
}

impl Config {
    /// Load configuration from multiple sources in priority order:
    /// 1. Command line arguments (highest priority)
    /// 2. Environment variables
    /// 3. Configuration file
    /// 4. Default values (lowest priority)
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        
        // Load from config file if it exists
        match Self::load_from_file("config.toml") {
            Ok(file_config) => {
                println!("✅ Configuration loaded from config.toml");
                config = file_config;
                config.app.version = env!("CARGO_PKG_VERSION").to_string();
            },
            Err(e) => {
                println!("⚠️  Warning: Could not load config.toml: {}", e);
                println!("📝 Using default configuration. You can create a config.toml file to customize settings.");
                // Continue with default config instead of exiting
            }
        }
        
        // Override with environment variables
        // config.load_from_env()?;
        
        // Validate the configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", path.as_ref()))?;
        
        Ok(config)
    }
    
    /// Load configuration overrides from environment variables
    pub fn _load_from_env(&mut self) -> Result<()> {
        // Database configuration
        if let Ok(host) = env::var("MYSQL_HOST") {
            self.database.host = host;
        }
        if let Ok(port) = env::var("MYSQL_PORT") {
            self.database.port = port.parse()
                .context("Invalid MYSQL_PORT value")?;
        }
        if let Ok(user) = env::var("MYSQL_USER") {
            self.database.user = user;
        }
        if let Ok(password) = env::var("MYSQL_PASSWORD") {
            self.database.password = password;
        }
        if let Ok(database) = env::var("MYSQL_DATABASE") {
            self.database.database = database;
        }
        
        // Server configuration
        if let Ok(host) = env::var("SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = env::var("SERVER_PORT") {
            self.server.port = port.parse()
                .context("Invalid SERVER_PORT value")?;
        }
        
        // Application configuration
        if let Ok(env) = env::var("APP_ENV") {
            self.app.environment = env;
        }
        if let Ok(debug) = env::var("APP_DEBUG") {
            self.app.debug = debug.parse()
                .context("Invalid APP_DEBUG value (expected true/false)")?;
        }
        
        // Logging configuration
        if let Ok(level) = env::var("LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(file_path) = env::var("LOG_FILE_PATH") {
            self.logging.file_path = PathBuf::from(file_path);
        }

        Ok(())
    }
    
    /// Validate the configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate database port
        if self.database.port == 0 {
            return Err(anyhow::anyhow!("Database port cannot be 0"));
        }
        
        // Validate server port
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }
        
        // Validate pool configuration
        if self.database.pool.min_connections > self.database.pool.max_connections {
            return Err(anyhow::anyhow!(
                "Database min_connections ({}) cannot be greater than max_connections ({})",
                self.database.pool.min_connections,
                self.database.pool.max_connections
            ));
        }
        
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid log level '{}'. Valid levels: {:?}",
                self.logging.level,
                valid_levels
            ));
        }
        
        // Validate environment
        let valid_envs = ["development", "staging", "production"];
        if !valid_envs.contains(&self.app.environment.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid environment '{}'. Valid environments: {:?}",
                self.app.environment,
                valid_envs
            ));
        }

        Ok(())
    }
    
    /// Save the current configuration to a TOML file
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path.as_ref()))?;
        
        Ok(())
    }
    
    /// Get database connection URL
    pub fn database_url(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.database.user,
            self.database.password,
            self.database.host,
            self.database.port,
            self.database.database
        )
    }
    
    /// Get server bind address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
    
    /// Check if running in production environment
    pub fn is_production(&self) -> bool {
        self.app.environment == "production"
    }
    
    /// Check if running in development environment
    pub fn is_development(&self) -> bool {
        self.app.environment == "development"
    }
    
    /// Get MySQL connection options with pool settings
    pub fn mysql_opts(&self) -> mysql::OptsBuilder {
        mysql::OptsBuilder::new()
            .ip_or_hostname(Some(&self.database.host))
            .tcp_port(self.database.port)
            .user(Some(&self.database.user))
            .pass(Some(&self.database.password))
            .db_name(Some(&self.database.database))
            // Production connection optimizations
            .tcp_connect_timeout(Some(std::time::Duration::from_secs(self.database.pool.connection_timeout)))
            .tcp_keepalive_time_ms(Some(600000)) // 10 minutes keepalive
            .tcp_nodelay(true) // Disable Nagle's algorithm for lower latency
            // Pool configuration applied via PoolOpts
            .pool_opts(Some(
                mysql::PoolOpts::new()
                    .with_constraints(
                        mysql::PoolConstraints::new(
                            self.database.pool.min_connections as usize,
                            self.database.pool.max_connections as usize
                        ).unwrap()
                    )
            ))
    }
    
    /// Create a MySQL connection pool
    pub fn create_connection_pool(&self) -> anyhow::Result<mysql::Pool> {
        let opts = self.mysql_opts();
        let pool = mysql::Pool::new(opts)
            .context("Failed to create MySQL connection pool")?;
        
        // Test the pool by getting a connection
        let _conn = pool.get_conn()
            .context("Failed to establish initial connection to MySQL")?;
        
        log::info!("MySQL connection pool created successfully");
        log::info!("Pool constraints: min={}, max={}", 
                  self.database.pool.min_connections, 
                  self.database.pool.max_connections);
        
        Ok(pool)
    }
}

/// Configuration builder for programmatic configuration creation
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }
    
    pub fn database_host(mut self, host: impl Into<String>) -> Self {
        self.config.database.host = host.into();
        self
    }
    
    pub fn database_port(mut self, port: u16) -> Self {
        self.config.database.port = port;
        self
    }
    
    pub fn database_user(mut self, user: impl Into<String>) -> Self {
        self.config.database.user = user.into();
        self
    }
    
    pub fn database_password(mut self, password: impl Into<String>) -> Self {
        self.config.database.password = password.into();
        self
    }
    
    pub fn database_name(mut self, database: impl Into<String>) -> Self {
        self.config.database.database = database.into();
        self
    }
    
    pub fn server_host(mut self, host: impl Into<String>) -> Self {
        self.config.server.host = host.into();
        self
    }
    
    pub fn server_port(mut self, port: u16) -> Self {
        self.config.server.port = port;
        self
    }
    
    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.config.logging.level = level.into();
        self
    }
    
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.config.app.environment = env.into();
        self
    }
    
    pub fn debug(mut self, debug: bool) -> Self {
        self.config.app.debug = debug;
        self
    }
    
    pub fn build(self) -> Result<Config> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 3306);
        assert_eq!(config.server.port, 3000);
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .database_host("example.com")
            .database_port(5432)
            .server_port(8080)
            .log_level("debug")
            .environment("production")
            .build()
            .unwrap();
        
        assert_eq!(config.database.host, "example.com");
        assert_eq!(config.database.port, 5432);
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.app.environment, "production");
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        config.database.port = 0;
        assert!(config.validate().is_err());
        
        config.database.port = 3306;
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_database_url() {
        let config = Config::default();
        let url = config.database_url();
        assert_eq!(url, "mysql://root:password@localhost:3306/mysql");
    }
    
    #[test]
    fn test_server_address() {
        let config = Config::default();
        let addr = config.server_address();
        assert_eq!(addr, "0.0.0.0:3000");
    }
}

