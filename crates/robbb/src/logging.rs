use opentelemetry_sdk::trace::{BatchConfig, RandomIdGenerator, Sampler};
use robbb_util::log_error;
use tracing_subscriber::{
    filter::FilterFn, prelude::__tracing_subscriber_SubscriberExt, EnvFilter,
};

/// Initializes tracing and logging configuration.
/// To configure tracing, set up the Opentelemetry tracing environment variables:
///
/// ```sh
/// export OTEL_SERVICE_NAME=robbb
/// export OTEL_EXPORTER_OTLP_PROTOCOL="http/protobuf"
/// export OTEL_EXPORTER_OTLP_ENDPOINT="https://otlp-gateway-prod-us-east-0.grafana.net/otlp"
/// export OTEL_EXPORTER_OTLP_HEADERS="Authorization=Basic <basic auth>"
/// ```
///
/// Will also respect the `RUST_LOG` environment variable for log filters.
pub fn init_tracing() {
    let log_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::try_new("robbb=trace,serenity=debug,serenity::http::ratelimiting=off,serenity::http::request=off")
                .unwrap()
        })
        .add_directive("robbb=trace".parse().unwrap());

    let remove_presence_update_filter = FilterFn::new(|metadata| {
        !(metadata.target() == "serenity::gateway::shard"
            && metadata.name() == "handle_gateway_dispatch"
            && metadata
                .fields()
                .field("event")
                .map_or(false, |event| event.to_string().starts_with("PresenceUpdate")))
    });
    let logfmt_builder = tracing_logfmt_otel::builder()
        .with_level(true)
        .with_target(true)
        .with_span_name(true)
        .with_span_path(true)
        .with_otel_data(true)
        .with_file(true)
        .with_line(true)
        .with_module(true);
    let sub = tracing_subscriber::registry().with(log_filter).with(remove_presence_update_filter);

    if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
        println!("Initializing opentelemetry");
        // TODO: check if we can decide sampling based on span name,
        // to have _some_ samples from regular stuff, but keep all commands etc
        let trace_config = opentelemetry_sdk::trace::config()
            .with_id_generator(RandomIdGenerator::default())
            .with_sampler(Sampler::AlwaysOn);
        // TODO: This is very low
        let batch_config = BatchConfig::default().with_max_export_batch_size(10);
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_trace_config(trace_config)
            .with_exporter(opentelemetry_otlp::new_exporter().http())
            .with_batch_config(batch_config)
            .install_batch(opentelemetry_sdk::runtime::Tokio);

        let tracer = match tracer {
            Ok(tracer) => tracer,
            Err(err) => {
                eprintln!("failed to initialize otel tracing: {err}");
                tracing::subscriber::set_global_default(sub)
                    .expect("setting default subscriber failed");
                return;
            }
        };

        let telemetry = tracing_opentelemetry::layer()
            .with_location(true)
            .with_threads(true)
            .with_tracked_inactivity(true)
            .with_tracer(tracer);

        tracing::info!("OTEL_EXPORTER_OTLP_ENDPOINT is set, initializing tracing layer");
        let sub = sub.with(telemetry).with(logfmt_builder.layer());
        tracing::subscriber::set_global_default(sub).expect("setting default subscriber failed");
    } else {
        tracing::info!("No OTEL_EXPORTER_OTLP_ENDPOINT is set, only initializing logging");
        let sub = sub.with(logfmt_builder.layer());
        tracing::subscriber::set_global_default(sub).expect("setting default subscriber failed");
    };
}

pub async fn send_honeycomb_deploy_marker(api_key: &str) {
    let version = robbb_util::util::bot_version();
    log_error!(
        reqwest::Client::new()
            .post("https://api.honeycomb.io/1/markers/robbb")
            .header("X-Honeycomb-Team", api_key)
            .body(format!(r#"{{"message": "{version}", "type": "deploy"}}"#,))
            .send()
            .await
    );
}
