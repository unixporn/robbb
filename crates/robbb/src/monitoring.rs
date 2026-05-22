use eyre::WrapErr as _;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
use serenity::{client, gateway::ConnectionStage};
use std::{net::SocketAddr, sync::Arc, time::Duration};

/// Counter – total poise/serenity events dispatched, labelled by `event`.
pub const EVENTS_TOTAL: &str = "robbb_events_total";

/// Counter – total non-bot messages received.
pub const MESSAGES_TOTAL: &str = "robbb_messages_total";

/// Counter – total messages deleted (single + bulk).
pub const MESSAGES_DELETED_TOTAL: &str = "robbb_messages_deleted_total";

/// Counter – total commands invoked, labelled by `command`.
pub const COMMANDS_TOTAL: &str = "robbb_commands_total";

/// Histogram – time in milliseconds between a message being created on Discord
/// and the bot beginning to process it (derived from the snowflake timestamp).
pub const MESSAGE_RECEIVE_LATENCY_MS: &str = "robbb_message_receive_latency_ms";

/// Gauge – number of guilds currently in the serenity cache.
pub const CACHE_GUILDS: &str = "robbb_cache_guilds";

/// Gauge – number of users currently in the serenity cache.
pub const CACHE_USERS: &str = "robbb_cache_users";

/// Gauge – last observed heartbeat latency per shard, in milliseconds.
pub const SHARD_LATENCY_MS: &str = "robbb_shard_latency_ms";

/// Gauge – 1.0 if the shard is `Connected`, 0.0 otherwise.
pub const SHARD_CONNECTED: &str = "robbb_shard_connected";

/// Counter – Discord HTTP rate-limits encountered, labelled by `path` and `global`.
pub const RATELIMITS_TOTAL: &str = "robbb_ratelimits_total";

/// Install the Prometheus metrics recorder and start the HTTP scrape endpoint.
///
/// The port is taken from the `METRICS_PORT` environment variable; it defaults
/// to `9090`.  The endpoint is served on `0.0.0.0:<port>/metrics`.
pub fn init_metrics() -> eyre::Result<()> {
    let port: u16 = std::env::var("METRICS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9090);

    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;

    PrometheusBuilder::new()
        // Millisecond-resolution buckets suitable for Discord message latency.
        .set_buckets_for_metric(
            Matcher::Full(MESSAGE_RECEIVE_LATENCY_MS.to_string()),
            &[5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0],
        )
        .wrap_err("Failed to configure histogram buckets for message receive latency")?
        .with_http_listener(addr)
        .install()
        .map_err(|e| eyre::eyre!("Failed to install Prometheus metrics exporter: {e}"))?;

    tracing::info!(metrics.port = port, "Prometheus metrics endpoint listening on :{port}");
    Ok(())
}

/// Spawn a background task that records cache and shard statistics every two
/// minutes.  Should be called once the serenity cache is ready.
pub fn start_cache_stats_task(
    ctx: client::Context,
    shard_manager: Arc<serenity::gateway::ShardManager>,
) {
    tokio::spawn(async move {
        // Give the cache a moment to warm up before the first collection.
        tokio::time::sleep(Duration::from_secs(30)).await;

        let mut interval = tokio::time::interval(Duration::from_secs(120));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            collect_cache_and_shard_stats(&ctx, &shard_manager).await;
        }
    });
}

async fn collect_cache_and_shard_stats(
    ctx: &client::Context,
    shard_manager: &serenity::gateway::ShardManager,
) {
    metrics::gauge!(CACHE_GUILDS).set(ctx.cache.guild_count() as f64);
    metrics::gauge!(CACHE_USERS).set(ctx.cache.user_count() as f64);

    let runners = shard_manager.runners.lock().await;
    for (shard_id, info) in runners.iter() {
        let shard_label = shard_id.0.to_string();

        if let Some(latency) = info.latency {
            metrics::gauge!(SHARD_LATENCY_MS, "shard_id" => shard_label.clone())
                .set(latency.as_secs_f64() * 1000.0);
        }

        let connected = matches!(info.stage, ConnectionStage::Connected);
        metrics::gauge!(SHARD_CONNECTED, "shard_id" => shard_label)
            .set(if connected { 1.0 } else { 0.0 });
    }
}
