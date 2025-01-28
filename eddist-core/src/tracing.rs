use metrics::{counter, describe_counter};
use tracing::field::Field;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter, Layer,
};

pub fn init_tracing() {
    let env_filter = EnvFilter::from_default_env();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false);

    let metrics_layer = MetricsLayer;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(metrics_layer)
        .init();

    describe_counter!("tracing_log_error", "error log count");
    describe_counter!("tracing_log_warn", "warn log count");
}

pub struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut target = None;

        // Extract fields from the event
        event.record(&mut |field: &Field, value: &dyn std::fmt::Debug| {
            let value = format!("{:?}", value);
            if field.name() == "log.target" {
                target = Some(value[1..value.len() - 1].to_string())
            }
        });

        let target = if let Some(target) = target {
            target
        } else {
            "unknown".to_string()
        };

        match event.metadata().level().as_str() {
            "ERROR" => {
                counter!("tracing_log_error", "target" => target).increment(1);
            }
            "WARN" => {
                counter!("tracing_log_warn", "target" => target).increment(1);
            }
            _ => {}
        }
    }
}
