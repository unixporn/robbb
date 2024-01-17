use opentelemetry_sdk::trace::{BatchConfig, RandomIdGenerator, Sampler};
use robbb_util::log_error;
use tracing_subscriber::{
    filter::{FilterExt, FilterFn},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
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
            EnvFilter::try_new("info,robbb=trace,serenity=debug,serenity::http::ratelimiting=off,serenity::http::request=off")
                .unwrap()
        })
        .add_directive("robbb=trace".parse().unwrap());
    let remove_presence_update_filter = FilterFn::new(|m| {
        !(m.target() == "serenity::gateway::shard"
            && m.name() == "handle_gateway_dispatch"
            && m.fields()
                .field("event")
                .map_or(false, |event| event.as_ref().starts_with("PresenceUpdate")))
    });
    let remove_recv_event_filter = EnvFilter::try_new(
        "trace,serenity::gateway::bridge::shard_runner[recv]=off,serenity::gateway::bridge::shard_runner[recv_event]=off"
    ).unwrap();

    let traces_extra_filter =
        EnvFilter::try_from_env("RUST_LOG_TRACES").unwrap_or_else(|_| EnvFilter::new("trace"));
    let remove_heartbeat_filter =
        EnvFilter::try_new("trace,serenity::gateway::ws[send_heartbeat]=off").unwrap();
    let remove_update_manager_filter =
        EnvFilter::try_new("trace,serenity::gateway::bridge::shard_runner[update_manager]=off")
            .unwrap();

    let logfmt_builder = tracing_logfmt_otel::builder()
        .with_level(true)
        .with_target(true)
        .with_span_name(true)
        .with_span_path(true)
        .with_otel_data(true)
        .with_file(true)
        .with_line(true)
        .with_module(true);
    let sub = tracing_subscriber::registry()
        .with(log_filter)
        .with(remove_presence_update_filter)
        .with(remove_recv_event_filter);

    if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
        opentelemetry::global::set_text_map_propagator(
            opentelemetry_sdk::propagation::TraceContextPropagator::new(),
        );
        println!("Initializing opentelemetry");
        // TODO: check if we can decide sampling based on span name,
        // to have _some_ samples from regular stuff, but keep all commands etc
        let trace_config = opentelemetry_sdk::trace::config()
            .with_id_generator(RandomIdGenerator::default())
            .with_sampler(Sampler::AlwaysOn);
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_trace_config(trace_config)
            .with_exporter(opentelemetry_otlp::new_exporter().http())
            .with_batch_config(BatchConfig::default())
            .install_batch(opentelemetry_sdk::runtime::Tokio);

        let tracer = match tracer {
            Ok(tracer) => tracer,
            Err(err) => {
                eprintln!("failed to initialize otel tracing: {err}");
                tracing::subscriber::set_global_default(sub.with(logfmt_builder.layer()))
                    .expect("setting default subscriber failed");
                return;
            }
        };

        let telemetry = tracing_opentelemetry::layer()
            .with_location(true)
            .with_threads(true)
            .with_tracked_inactivity(true)
            .with_tracer(tracer)
            .with_filter(remove_heartbeat_filter)
            .with_filter(remove_update_manager_filter)
            .with_filter(traces_extra_filter);

        println!("OTEL_EXPORTER_OTLP_ENDPOINT is set, initializing tracing layer");
        sub.with(telemetry).with(logfmt_builder.layer()).init();
    } else {
        println!("No OTEL_EXPORTER_OTLP_ENDPOINT is set, only initializing logging");
        sub.with(logfmt_builder.layer()).init();
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
