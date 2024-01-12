use robbb_util::log_error;
use tracing_subscriber::{
    filter::FilterFn, prelude::__tracing_subscriber_SubscriberExt, EnvFilter,
};

pub fn init_tracing(honeycomb_api_key: Option<String>) {
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

    let sub = tracing_subscriber::registry::Registry::default()
        .with(log_filter)
        .with(remove_presence_update_filter)
        .with(
            tracing_logfmt::builder()
                .with_level(true)
                .with_target(true)
                .with_span_name(true)
                .with_span_path(true)
                .layer()
                .with_ansi(true),
        );

    if let Some(api_key) = honeycomb_api_key {
        tracing::info!("honeycomb api key is set, initializing honeycomb layer");
        let config = libhoney::Config {
            options: libhoney::client::Options {
                api_key,
                dataset: "robbb".to_string(),
                ..libhoney::client::Options::default()
            },
            transmission_options: libhoney::transmission::Options::default(),
        };

        let honeycomb_layer = tracing_honeycomb::Builder::new_libhoney("robbb", config).build();

        let sub = sub.with(honeycomb_layer);
        tracing::subscriber::set_global_default(sub).expect("setting default subscriber failed");
    } else {
        tracing::info!("no honeycomb api key is set");
        let sub = sub.with(tracing_honeycomb::new_blackhole_telemetry_layer());
        tracing::subscriber::set_global_default(sub).expect("setting default subscriber failed");
    };
}

pub async fn send_honeycomb_deploy_marker(api_key: &str) {
    let client = reqwest::Client::new();
    log_error!(
        client
            .post("https://api.honeycomb.io/1/markers/robbb")
            .header("X-Honeycomb-Team", api_key)
            .body(format!(
                r#"{{"message": "{}", "type": "deploy"}}"#,
                robbb_util::util::bot_version()
            ))
            .send()
            .await
    );
}
