use std::{collections::HashMap};
use serde::{Deserialize};

#[derive(Debug, Deserialize)]
pub struct UpstreamConfig {
    pub http_url: String,
    pub rate_limit: Option<String>,
    pub failover: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub exclude_methods: Option<HashMap<String, bool>>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub cache: CacheConfig,
    pub upstreams: Vec<UpstreamConfig>,
    pub try_next_upstream_on_errors: Option<HashMap<String, bool>>,
}