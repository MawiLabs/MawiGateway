//! Shared HTTP client for all outbound provider + health-check calls.
//!
//! Why this exists:
//!   - `reqwest::Client::new()` per call pays a fresh TCP + TLS handshake
//!     every request. With chat / image / video providers all making their
//!     own calls, this added 50-150ms of pure handshake to p99 latency.
//!   - HTTP/2 connection re-use across the process means one keep-alive
//!     connection per provider host can serve thousands of concurrent
//!     requests.
//!
//! Use:
//!
//!   use mawi_core::http;
//!   let client = http::shared_client();
//!   let response = client.get(url).send().await?;
//!
//! The client is configured for:
//!   - 30s connect timeout (generous for slow networks)
//!   - 120s overall request timeout (cap for non-streaming calls; streams
//!     should explicitly opt out via `.timeout(Duration::MAX)` per call)
//!   - HTTP/2 with keep-alive pings every 30s
//!   - Connection pool: 100 idle per host, 5min idle timeout
//!   - User-Agent identifying us as the gateway for upstream observability

use std::sync::OnceLock;
use std::time::Duration;

static SHARED_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Return the process-wide shared `reqwest::Client`. Cheap (Arc clone via
/// reqwest's internal handle); call freely.
pub fn shared_client() -> &'static reqwest::Client {
    SHARED_CLIENT.get_or_init(build_client)
}

fn build_client() -> reqwest::Client {
    let user_agent = format!(
        "mawi-gateway/{} (+https://github.com/MawiLabs/MawiGateway)",
        env!("CARGO_PKG_VERSION")
    );

    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .pool_idle_timeout(Duration::from_secs(300))
        .pool_max_idle_per_host(100)
        // HTTP/2 keep-alive pings keep long-lived connections healthy
        // through aggressive proxy timeouts.
        .http2_keep_alive_interval(Duration::from_secs(30))
        .http2_keep_alive_timeout(Duration::from_secs(10))
        .http2_keep_alive_while_idle(true)
        .tcp_nodelay(true)
        .build()
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "failed to build shared HTTP client; falling back to default");
            // Last resort: default client. Better than panic since this
            // runs at first request, not at boot.
            reqwest::Client::new()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_client_returns_same_instance() {
        let a = shared_client();
        let b = shared_client();
        // Same `&'static` so this should be the same address.
        assert!(std::ptr::eq(a, b));
    }
}
