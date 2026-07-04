use std::borrow::Cow;

use crate::error::AppError;
use redis::AsyncCommands;
use serde::Serialize;
use tracing::{debug, warn};

const MAX_PATH_LEN: usize = 200;
const KEY_NS: &str = "busrzi";

/// Index Pages
const INDEX: &[&str] = &[
    "/index.html",
    "/index.htm",
    "/index.php",
    "/index.aspx",
    "/index",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CounterData {
    pub site_uv: i64,
    pub site_pv: i64,
    pub page_pv: i64,
}

#[derive(Clone)]
pub struct CounterService {
    redis: redis::aio::ConnectionManager,
    ttl_seconds: Option<i64>,
}

impl std::fmt::Debug for CounterService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterService")
            .field("ttl_seconds", &self.ttl_seconds)
            .finish_non_exhaustive()
    }
}

impl CounterService {
    pub fn new(redis: redis::aio::ConnectionManager, ttl_days: u64) -> Self {
        Self {
            redis,
            ttl_seconds: if ttl_days == 0 {
                None
            } else {
                Some((ttl_days * 24 * 60 * 60) as i64)
            },
        }
    }

    /// 记录一次页面访问，返回最新的站点 UV、站点 PV、页面 PV。
    #[tracing::instrument(skip_all, fields(host = %target.host, path = %target.path, is_new_uv))]
    pub async fn record(&self, target: &Target, is_new_uv: bool) -> Result<CounterData, AppError> {
        let mut conn = self.redis.clone();

        let site_pv_key = site_pv_key(&target.host);
        let page_pv_key = page_pv_key(target);
        let page_inventory_key = page_inventory_key(&target.host);
        let site_uv_key = site_uv_key(&target.host);

        // 先原子地递增站点 PV 和页面 PV；若配置了 TTL 则同时刷新。
        let (site_pv, page_pv) = match self.ttl_seconds {
            Some(ttl) => {
                let (site_pv, _, page_pv, _, _, _): (i64, (), i64, (), (), ()) = match redis::pipe()
                    .incr(&site_pv_key, 1i64)
                    .expire(&site_pv_key, ttl)
                    .incr(&page_pv_key, 1i64)
                    .expire(&page_pv_key, ttl)
                    .sadd(&page_inventory_key, &target.path)
                    .expire(&page_inventory_key, ttl)
                    .query_async(&mut conn)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(error = %e, "failed to update page/site pv counters");
                        return Err(e.into());
                    }
                };
                (site_pv, page_pv)
            }
            None => {
                let (site_pv, page_pv, _): (i64, i64, ()) = match redis::pipe()
                    .incr(&site_pv_key, 1i64)
                    .incr(&page_pv_key, 1i64)
                    .sadd(&page_inventory_key, &target.path)
                    .query_async(&mut conn)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(error = %e, "failed to update page/site pv counters");
                        return Err(e.into());
                    }
                };
                (site_pv, page_pv)
            }
        };

        // 处理站点 UV：新的独立访客递增
        let site_uv = if is_new_uv {
            match self.ttl_seconds {
                Some(ttl) => {
                    let result: Result<(i64, ()), redis::RedisError> = redis::pipe()
                        .incr(&site_uv_key, 1i64)
                        .expire(&site_uv_key, ttl)
                        .query_async(&mut conn)
                        .await;
                    match result {
                        Ok((uv, _)) => uv,
                        Err(e) => {
                            warn!(error = %e, "failed to update site uv counter");
                            return Err(e.into());
                        }
                    }
                }
                None => {
                    let result: Result<(i64,), redis::RedisError> = redis::pipe()
                        .incr(&site_uv_key, 1i64)
                        .query_async(&mut conn)
                        .await;
                    match result {
                        Ok((uv,)) => uv,
                        Err(e) => {
                            warn!(error = %e, "failed to update site uv counter");
                            return Err(e.into());
                        }
                    }
                }
            }
        } else {
            conn.get(&site_uv_key).await.unwrap_or(0)
        };

        debug!(
            site_pv = site_pv,
            page_pv = page_pv,
            site_uv = site_uv,
            "counter values fetched"
        );

        Ok(CounterData {
            site_uv,
            site_pv,
            page_pv,
        })
    }
}

/// 从完整 URL 解析并规范化出计数目标
pub fn target_from_url(url_str: &str) -> Result<Target, AppError> {
    let url = url::Url::parse(url_str).map_err(|e| AppError::InvalidUrl(e.to_string()))?;
    let host = url
        .host_str()
        .ok_or_else(|| AppError::InvalidUrl("missing host".into()))?;
    Ok(normalize_target(host, url.path()))
}

/// URL 规范化
pub fn normalize_target(host: &str, path: &str) -> Target {
    let host = host.trim().to_lowercase();

    // 归一化前导 `/`，为空则视为根路径
    let trimmed = path.trim();
    let mut path: Cow<str> = match trimmed {
        "" => Cow::Borrowed("/"),
        p if p.starts_with('/') => Cow::Borrowed(p),
        p => Cow::Owned(format!("/{p}")),
    };

    // 完整匹配 index 文件 -> 根路径
    if INDEX.contains(&path.as_ref()) {
        path = Cow::Borrowed("/");
    } else if let Some(suffix) = INDEX.iter().find(|s| path.ends_with(**s)) {
        // 去掉 "/index*" 后缀
        let head = &path[..path.len() - suffix.len()];
        path = Cow::Owned(if head.is_empty() {
            "/".into()
        } else {
            head.into()
        });
    }

    // 去掉多余的尾部斜杠（根路径除外）
    if path != "/" {
        let stripped = path.trim_end_matches('/');
        let stripped = if stripped.is_empty() { "/" } else { stripped };
        if stripped.len() != path.len() {
            path = Cow::Owned(stripped.to_string());
        }
    }

    // 按字节长度截断，在合法的 UTF-8 字符边界上
    let path = if path.len() > MAX_PATH_LEN {
        let mut end = MAX_PATH_LEN;
        while !path.is_char_boundary(end) {
            end -= 1;
        }
        path[..end].to_string()
    } else {
        path.into_owned()
    };

    Target { host, path }
}

fn site_uv_key(host: &str) -> String {
    format!("{KEY_NS}:site:uv:{host}")
}

fn site_pv_key(host: &str) -> String {
    format!("{KEY_NS}:site:pv:{host}")
}

fn page_pv_key(target: &Target) -> String {
    format!("{KEY_NS}:page:pv:{}:{}", target.host, target.path)
}

fn page_inventory_key(host: &str) -> String {
    format!("{KEY_NS}:site:pages:{host}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_target_basic() {
        assert_eq!(
            normalize_target("EXAMPLE.COM", "/blog/post"),
            Target {
                host: "example.com".into(),
                path: "/blog/post".into(),
            }
        );
    }

    #[test]
    fn test_normalize_target_index_files() {
        let cases = [
            ("/index.html", "/"),
            ("/index.htm", "/"),
            ("/index.php", "/"),
            ("/index.aspx", "/"),
            ("/index", "/"),
            ("/blog/index.html", "/blog"),
            ("/blog/index.htm", "/blog"),
            ("/blog/index.php", "/blog"),
            ("/blog/index.aspx", "/blog"),
            ("/blog/index", "/blog"),
            ("/a/b/c/index.html", "/a/b/c"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                normalize_target("example.com", input),
                Target {
                    host: "example.com".into(),
                    path: expected.into(),
                },
                "failed for input {input}"
            );
        }
    }

    #[test]
    fn test_normalize_target_trailing_slash() {
        let cases = [
            ("/blog/", "/blog"),
            ("/blog///", "/blog"),
            ("/", "/"),
            ("///", "/"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                normalize_target("example.com", input),
                Target {
                    host: "example.com".into(),
                    path: expected.into(),
                },
                "failed for input {input}"
            );
        }
    }

    #[test]
    fn test_normalize_target_root() {
        assert_eq!(
            normalize_target("example.com", "/"),
            Target {
                host: "example.com".into(),
                path: "/".into(),
            }
        );
    }

    #[test]
    fn test_normalize_target_empty_and_whitespace() {
        assert_eq!(
            normalize_target("example.com", ""),
            Target {
                host: "example.com".into(),
                path: "/".into(),
            }
        );
        assert_eq!(
            normalize_target("  EXAMPLE.COM  ", "  /blog/  "),
            Target {
                host: "example.com".into(),
                path: "/blog".into(),
            }
        );
    }

    #[test]
    fn test_normalize_target_no_leading_slash() {
        assert_eq!(
            normalize_target("example.com", "blog/post"),
            Target {
                host: "example.com".into(),
                path: "/blog/post".into(),
            }
        );
    }

    #[test]
    fn test_normalize_target_multiple_slashes() {
        assert_eq!(
            normalize_target("example.com", "//blog///post//"),
            Target {
                host: "example.com".into(),
                path: "//blog///post".into(),
            }
        );
    }

    #[test]
    fn test_normalize_target_length_truncation() {
        let long_path = format!("/{}", "a".repeat(300));
        let target = normalize_target("example.com", &long_path);
        assert_eq!(target.path.len(), MAX_PATH_LEN);
        assert!(target.path.starts_with('/'));
        assert!(target.path.is_char_boundary(MAX_PATH_LEN));
    }

    #[test]
    fn test_normalize_target_unicode_truncation() {
        // 中文字符每个 3 字节，验证截断不会破坏 UTF-8 字符边界。
        let chinese_path = format!("/博客{}", "中".repeat(100));
        let target = normalize_target("example.com", &chinese_path);
        assert!(target.path.starts_with('/'));
        assert!(target.path.len() <= MAX_PATH_LEN);
        assert!(target.path.is_char_boundary(target.path.len()));
    }

    #[test]
    fn test_target_from_url_parses_port() {
        let target = target_from_url("https://example.com:443/blog/index.html").unwrap();
        assert_eq!(target.host, "example.com");
        assert_eq!(target.path, "/blog");
    }

    #[test]
    fn test_target_from_url_nonstandard_port() {
        let target = target_from_url("https://example.com:8443/blog/post").unwrap();
        assert_eq!(target.host, "example.com");
        assert_eq!(target.path, "/blog/post");
    }

    #[test]
    fn test_target_from_url_ignores_query_and_fragment() {
        let target = target_from_url("https://example.com/blog/post?id=1&page=2#section").unwrap();
        assert_eq!(target.host, "example.com");
        assert_eq!(target.path, "/blog/post");
    }

    #[test]
    fn test_target_from_url_ignores_auth() {
        let target = target_from_url("https://user:pass@example.com/blog/post").unwrap();
        assert_eq!(target.host, "example.com");
        assert_eq!(target.path, "/blog/post");
    }

    #[test]
    fn test_target_from_url_various_schemes() {
        for url in ["http://example.com/blog", "https://example.com/blog"] {
            let target = target_from_url(url).unwrap();
            assert_eq!(target.host, "example.com");
            assert_eq!(target.path, "/blog");
        }
    }

    #[test]
    fn test_target_from_url_rejects_invalid() {
        assert!(target_from_url("not-a-url").is_err());
        assert!(target_from_url("").is_err());
        assert!(target_from_url("ftp://example.com/blog").is_ok()); // url crate 接受 ftp
        assert!(target_from_url("file:///etc/passwd").is_err()); // 无 host
    }

    #[test]
    fn test_key_builders() {
        let target = Target {
            host: "example.com".into(),
            path: "/blog/post".into(),
        };
        assert_eq!(site_uv_key("example.com"), "busrzi:site:uv:example.com");
        assert_eq!(site_pv_key("example.com"), "busrzi:site:pv:example.com");
        assert_eq!(
            page_pv_key(&target),
            "busrzi:page:pv:example.com:/blog/post"
        );
        assert_eq!(
            page_inventory_key("example.com"),
            "busrzi:site:pages:example.com"
        );
    }
}
