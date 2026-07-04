use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    /// 监听地址，例如 "0.0.0.0:8080"
    pub addr: String,
    /// Redis/Valkey 连接 URL
    pub redis_url: String,
    /// 客户端 JS 文件路径
    pub script_path: String,
    /// 计数器数据 TTL，单位天，默认 90
    pub ttl_days: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let port = env::var("PORT")
            .or_else(|_| env::var("API_PORT"))
            .unwrap_or_else(|_| "8080".to_string());

        let addr = format!("0.0.0.0:{port}");

        let redis_url = env::var("REDIS_URL")
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or("REDIS_URL must be set")?;

        let script_path = env::var("SCRIPT_PATH")
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "assets/client.js".to_string());

        let ttl_days = env::var("TTL_DAYS")
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
            .unwrap_or(90);

        Ok(Config {
            addr,
            redis_url,
            script_path,
            ttl_days,
        })
    }
}

