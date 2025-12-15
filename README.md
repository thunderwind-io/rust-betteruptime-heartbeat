# betteruptime-heartbeat

Better Uptime heartbeat monitoring client for Rust services.

This crate provides a simple, non-blocking way to send periodic heartbeat pings to [Better Uptime](https://betteruptime.com/) monitoring service. It spawns a background tokio task that sends HTTP GET requests at configured intervals.

## Features

- **Environment-based configuration** with sensible defaults
- **Non-blocking** tokio async runtime
- **Automatic error handling** and retry (never panics)
- **Structured logging** via `tracing`
- **Zero-dependency security**: uses `rustls-tls` (no OpenSSL)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
betteruptime-heartbeat = { git = "https://github.com/thunderwind-net/rust-betteruptime-heartbeat" }
```

## Usage

### Quick Start

At the start of your service (after initializing tracing):

```rust,no_run
#[tokio::main]
async fn main() {
    // Initialize your tracing subscriber
    tracing_subscriber::fmt::init();

    // Start heartbeat monitoring
    betteruptime_heartbeat::spawn_from_env();

    // Rest of your service startup...
}
```

### Environment Variables

Configure heartbeat monitoring through environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `HEARTBEAT_URL` | Yes | - | Better Uptime heartbeat URL from your monitor |
| `HEARTBEAT_INTERVAL_SECS` | No | `60` | Interval between heartbeats in seconds |
| `HEARTBEAT_TIMEOUT_SECS` | No | `10` | HTTP request timeout in seconds |

### Example Configuration

```bash
# Required: URL from Better Uptime heartbeat monitor
HEARTBEAT_URL=https://uptime.betterstack.com/api/v1/heartbeat/<YOUR_TOKEN>

# Optional: Send heartbeat every 2 minutes
HEARTBEAT_INTERVAL_SECS=120

# Optional: 15 second timeout
HEARTBEAT_TIMEOUT_SECS=15
```

### Advanced Usage

If you need to configure the heartbeat programmatically:

```rust,no_run
use betteruptime_heartbeat::{HeartbeatConfig, spawn};

#[tokio::main]
async fn main() {
    let config = HeartbeatConfig {
        url: "https://uptime.betterstack.com/api/v1/heartbeat/TOKEN".to_string(),
        interval_secs: 60,
        timeout_secs: 10,
    };

    spawn(config);
}
```

## Behavior

- If `HEARTBEAT_URL` is not set or empty, heartbeat is **disabled** and a log message is emitted
- The heartbeat task runs in the background and never blocks your service
- **Errors never panic**: network failures and non-2xx responses are logged at `warn` level
- Successful heartbeats are logged at `debug` level
- The task spawns once and runs for the lifetime of your process

## Better Uptime Setup

1. Log in to [Better Uptime](https://betteruptime.com/)
2. Create a new **Heartbeat** monitor
3. Set the **expected interval** (e.g., 2 minutes)
4. Set a **grace period** (e.g., 2 minutes)
5. Copy the heartbeat URL
6. Set the `HEARTBEAT_URL` environment variable in your deployment

## License

MIT

## Contributing

This crate is part of the [Thunderwind](https://github.com/thunderwind-net) confidential computing platform.

Issues and pull requests are welcome at the [GitHub repository](https://github.com/thunderwind-net/rust-betteruptime-heartbeat).
