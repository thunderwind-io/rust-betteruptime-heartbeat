#![deny(
    unsafe_code,
    warnings,
    missing_docs,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#![allow(clippy::multiple_crate_versions)] // External dependencies may use different versions
#![doc = include_str!("../README.md")]

//! Better Uptime heartbeat monitoring client for Rust services.
//!
//! This crate provides a simple, non-blocking way to send periodic heartbeat
//! pings to Better Uptime monitoring service. It spawns a background task
//! that sends HTTP GET requests at configured intervals.
//!
//! # Features
//!
//! - Environment-based configuration with sensible defaults
//! - Non-blocking tokio async runtime
//! - Automatic error handling and retry (never panics)
//! - Structured logging via `tracing`
//!
//! # Example
//!
//! ```rust,no_run
//! #[tokio::main]
//! async fn main() {
//!     // At the start of your service (after tracing is initialized):
//!     betteruptime_heartbeat::spawn_from_env();
//! }
//! ```

use std::time::Duration;

/// Configuration for heartbeat client.
///
/// # Example
///
/// ```rust
/// use betteruptime_heartbeat::HeartbeatConfig;
///
/// let config = HeartbeatConfig {
///     url: "https://uptime.betterstack.com/api/v1/heartbeat/TOKEN".to_string(),
///     interval_secs: 60,
///     timeout_secs: 10,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Better Uptime heartbeat URL.
    pub url: String,
    /// Interval between heartbeats in seconds (default: 60).
    pub interval_secs: u64,
    /// HTTP request timeout in seconds (default: 10).
    pub timeout_secs: u64,
}

impl HeartbeatConfig {
    /// Create config from environment variables.
    ///
    /// Returns `None` if `HEARTBEAT_URL` is not set or empty.
    ///
    /// # Environment variables
    ///
    /// - `HEARTBEAT_URL` (required): Better Uptime heartbeat URL
    /// - `HEARTBEAT_INTERVAL_SECS` (optional): interval in seconds, default 60
    /// - `HEARTBEAT_TIMEOUT_SECS` (optional): timeout in seconds, default 10
    ///
    /// # Example
    ///
    /// ```rust
    /// use betteruptime_heartbeat::HeartbeatConfig;
    ///
    /// if let Some(config) = HeartbeatConfig::from_env() {
    ///     println!("Heartbeat URL: {}", config.url);
    /// }
    /// ```
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("HEARTBEAT_URL").ok()?;

        if url.trim().is_empty() {
            return None;
        }

        let interval_secs = std::env::var("HEARTBEAT_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        let timeout_secs =
            std::env::var("HEARTBEAT_TIMEOUT_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);

        Some(Self { url, interval_secs, timeout_secs })
    }
}

/// Spawn heartbeat background task if configured.
///
/// Returns `true` if task was spawned, `false` if disabled.
///
/// This function reads configuration from environment variables via
/// [`HeartbeatConfig::from_env()`] and spawns a background task if
/// `HEARTBEAT_URL` is configured.
///
/// # Example
///
/// ```rust,no_run
/// #[tokio::main]
/// async fn main() {
///     // At service startup:
///     if betteruptime_heartbeat::spawn_from_env() {
///         println!("Heartbeat monitoring enabled");
///     } else {
///         println!("Heartbeat monitoring disabled");
///     }
/// }
/// ```
#[must_use]
pub fn spawn_from_env() -> bool {
    HeartbeatConfig::from_env().map_or_else(
        || {
            tracing::info!("HEARTBEAT_URL not configured, heartbeat disabled");
            false
        },
        |config| {
            spawn(config);
            true
        },
    )
}

/// Spawn heartbeat background task with explicit config.
///
/// This function creates an HTTP client and spawns a background tokio task
/// that sends periodic heartbeat pings to the configured URL.
///
/// # Example
///
/// ```rust,no_run
/// use betteruptime_heartbeat::{HeartbeatConfig, spawn};
///
/// #[tokio::main]
/// async fn main() {
///     let config = HeartbeatConfig {
///         url: "https://uptime.betterstack.com/api/v1/heartbeat/TOKEN".to_string(),
///         interval_secs: 60,
///         timeout_secs: 10,
///     };
///
///     spawn(config);
/// }
/// ```
pub fn spawn(config: HeartbeatConfig) {
    tracing::info!(
        "Heartbeat task spawned: interval={}s, timeout={}s",
        config.interval_secs,
        config.timeout_secs
    );

    tokio::spawn(async move {
        heartbeat_loop(config).await;
    });
}

/// Internal heartbeat loop that runs indefinitely.
///
/// Sends GET requests to the configured URL at regular intervals.
/// Never panics - all errors are logged and the loop continues.
async fn heartbeat_loop(config: HeartbeatConfig) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create HTTP client for heartbeat: {}", e);
            return;
        }
    };

    let mut interval = tokio::time::interval(Duration::from_secs(config.interval_secs));

    // First tick completes immediately, skip it to align with intended interval
    interval.tick().await;

    loop {
        interval.tick().await;

        match client.get(&config.url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    tracing::debug!("Heartbeat sent successfully");
                } else {
                    tracing::warn!(
                        "Heartbeat request returned non-2xx status: {}",
                        response.status()
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Heartbeat request failed: {}", e);
            }
        }
    }
}

#[cfg(test)]
#[allow(unsafe_code)] // Tests need to manipulate environment variables
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_config_from_env_returns_none_when_url_not_set() {
        // Clear environment
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
            std::env::remove_var("HEARTBEAT_INTERVAL_SECS");
            std::env::remove_var("HEARTBEAT_TIMEOUT_SECS");
        }

        let config = HeartbeatConfig::from_env();
        assert!(config.is_none());
    }

    #[test]
    #[serial]
    fn test_config_from_env_returns_none_when_url_is_empty() {
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::set_var("HEARTBEAT_URL", "");
        }

        let config = HeartbeatConfig::from_env();
        assert!(config.is_none());

        // SAFETY: Cleanup
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
        }
    }

    #[test]
    #[serial]
    fn test_config_from_env_returns_none_when_url_is_whitespace() {
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::set_var("HEARTBEAT_URL", "   ");
        }

        let config = HeartbeatConfig::from_env();
        assert!(config.is_none());

        // SAFETY: Cleanup
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
        }
    }

    #[test]
    #[serial]
    fn test_config_from_env_uses_defaults() {
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::set_var("HEARTBEAT_URL", "https://example.com/heartbeat");
            std::env::remove_var("HEARTBEAT_INTERVAL_SECS");
            std::env::remove_var("HEARTBEAT_TIMEOUT_SECS");
        }

        let config = HeartbeatConfig::from_env().expect("config should be Some");

        assert_eq!(config.url, "https://example.com/heartbeat");
        assert_eq!(config.interval_secs, 60);
        assert_eq!(config.timeout_secs, 10);

        // SAFETY: Cleanup
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
        }
    }

    #[test]
    #[serial]
    fn test_config_from_env_parses_custom_values() {
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::set_var("HEARTBEAT_URL", "https://example.com/custom");
            std::env::set_var("HEARTBEAT_INTERVAL_SECS", "120");
            std::env::set_var("HEARTBEAT_TIMEOUT_SECS", "30");
        }

        let config = HeartbeatConfig::from_env().expect("config should be Some");

        assert_eq!(config.url, "https://example.com/custom");
        assert_eq!(config.interval_secs, 120);
        assert_eq!(config.timeout_secs, 30);

        // SAFETY: Cleanup
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
            std::env::remove_var("HEARTBEAT_INTERVAL_SECS");
            std::env::remove_var("HEARTBEAT_TIMEOUT_SECS");
        }
    }

    #[test]
    #[serial]
    fn test_config_from_env_ignores_invalid_interval() {
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::remove_var("HEARTBEAT_INTERVAL_SECS");
            std::env::remove_var("HEARTBEAT_TIMEOUT_SECS");
            std::env::set_var("HEARTBEAT_URL", "https://example.com/heartbeat");
            std::env::set_var("HEARTBEAT_INTERVAL_SECS", "invalid");
        }

        let config = HeartbeatConfig::from_env().expect("config should be Some");

        // Should fallback to default
        assert_eq!(config.interval_secs, 60);

        // SAFETY: Cleanup
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
            std::env::remove_var("HEARTBEAT_INTERVAL_SECS");
        }
    }

    #[test]
    #[serial]
    fn test_config_from_env_ignores_invalid_timeout() {
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::remove_var("HEARTBEAT_INTERVAL_SECS");
            std::env::remove_var("HEARTBEAT_TIMEOUT_SECS");
            std::env::set_var("HEARTBEAT_URL", "https://example.com/heartbeat");
            std::env::set_var("HEARTBEAT_TIMEOUT_SECS", "not-a-number");
        }

        let config = HeartbeatConfig::from_env().expect("config should be Some");

        // Should fallback to default
        assert_eq!(config.timeout_secs, 10);

        // SAFETY: Cleanup
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
            std::env::remove_var("HEARTBEAT_TIMEOUT_SECS");
        }
    }

    #[test]
    #[serial]
    fn test_spawn_from_env_returns_false_when_not_configured() {
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
        }

        let spawned = spawn_from_env();
        assert!(!spawned);
    }

    #[tokio::test]
    #[serial]
    async fn test_spawn_from_env_returns_true_when_configured() {
        // SAFETY: Tests run sequentially and we clean up after ourselves
        unsafe {
            std::env::set_var("HEARTBEAT_URL", "https://example.com/heartbeat");
        }

        let spawned = spawn_from_env();
        assert!(spawned);

        // SAFETY: Cleanup
        unsafe {
            std::env::remove_var("HEARTBEAT_URL");
        }
    }
}
