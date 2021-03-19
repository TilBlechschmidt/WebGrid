use super::structures::{Metric, MetricType, MetricValue};
use crate::libraries::helpers::keys;
use crate::libraries::metrics::SESSION_STARTUP_HISTOGRAM_BUCKETS;
use redis::{aio::ConnectionLike, AsyncCommands};

static HTTP_METHODS: [&str; 9] = [
    "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "CONNECT", "PATCH", "TRACE",
];
static LOG_LEVELS: [&str; 3] = ["INFO", "WARN", "FAIL"];

pub async fn traffic_value<C: AsyncCommands + ConnectionLike>(
    con: &mut C,
    direction: &str,
) -> MetricValue {
    let bytes: f64 = con
        .hget(&*keys::metrics::http::NET_BYTES_TOTAL, direction)
        .await
        .unwrap_or_default();

    MetricValue::from_value(bytes).with_label("direction", direction)
}

#[allow(clippy::eval_order_dependence)]
pub async fn proxy_traffic<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Metric {
    Metric {
        name: "webgrid_proxy_http_net_bytes_total".to_string(),
        description: "Bytes (body only) transferred through all proxy instances".to_string(),

        metric_type: MetricType::Counter,
        values: vec![
            traffic_value(con, "in").await,
            traffic_value(con, "out").await,
        ],
    }
}

pub async fn proxy_requests<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Metric {
    let mut metric = Metric {
        name: "webgrid_proxy_http_requests_total".to_string(),
        description: "Requests processed by all proxy instances".to_string(),

        metric_type: MetricType::Counter,
        values: Vec::new(),
    };

    for method in HTTP_METHODS.iter() {
        if let Ok(status_codes) = con
            .hgetall::<_, Vec<(String, f64)>>(keys::metrics::http::requests_total(method))
            .await
        {
            for (status_code, count) in status_codes {
                metric.values.push(
                    MetricValue::from_value(count)
                        .with_label("method", method)
                        .with_label("code", &status_code),
                );
            }
        }
    }

    metric
}

pub async fn session_log<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Metric {
    let mut metric = Metric {
        name: "webgrid_session_status_codes_total".to_string(),
        description: "Log codes for sessions".to_string(),

        metric_type: MetricType::Counter,
        values: Vec::new(),
    };

    for log_level in LOG_LEVELS.iter() {
        if let Ok(log_codes) = con
            .hgetall::<_, Vec<(String, f64)>>(keys::metrics::session::log(log_level))
            .await
        {
            for (log_code, count) in log_codes {
                metric.values.push(
                    MetricValue::from_value(count)
                        .with_label("level", log_level)
                        .with_label("code", &log_code),
                );
            }
        }
    }

    metric
}

pub async fn session_startup_duration<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Metric {
    let mut metric = Metric {
        name: "webgrid_session_startup_duration_seconds".to_string(),
        description: "Total startup duration of a session including queue and scheduling"
            .to_string(),

        metric_type: MetricType::Histogram,
        values: Vec::new(),
    };

    for bucket in SESSION_STARTUP_HISTOGRAM_BUCKETS.iter() {
        let bucket_name = bucket.to_string();
        let bucket_value = con
            .hget(
                &*keys::metrics::session::startup_histogram::BUCKETS,
                &bucket_name,
            )
            .await
            .unwrap_or_default();

        metric.values.push(
            MetricValue::from_value(bucket_value)
                .with_label("le", &bucket_name)
                .with_name_postfix("_bucket"),
        );
    }

    let infinity_bucket_value = con
        .hget(&*keys::metrics::session::startup_histogram::BUCKETS, "+Inf")
        .await
        .unwrap_or_default();
    metric.values.push(
        MetricValue::from_value(infinity_bucket_value)
            .with_label("le", "+Inf")
            .with_name_postfix("_bucket"),
    );

    let sum_value = con
        .get(&*keys::metrics::session::startup_histogram::SUM)
        .await
        .unwrap_or_default();
    metric
        .values
        .push(MetricValue::from_value(sum_value).with_name_postfix("_sum"));

    let sum_value = con
        .get(&*keys::metrics::session::startup_histogram::COUNT)
        .await
        .unwrap_or_default();
    metric
        .values
        .push(MetricValue::from_value(sum_value).with_name_postfix("_count"));

    metric
}

pub async fn sessions_active<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Metric {
    Metric {
        name: "webgrid_sessions_active".to_string(),
        description: "Current number of active sessions (including queued and pending)".to_string(),

        metric_type: MetricType::Gauge,
        values: vec![MetricValue::from_value(
            con.scard(&*keys::session::LIST_ACTIVE)
                .await
                .unwrap_or_default(),
        )],
    }
}

pub async fn sessions_terminated<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Metric {
    Metric {
        name: "webgrid_sessions_terminated".to_string(),
        description:
            "Current number of terminated sessions that have not yet been purged from the database"
                .to_string(),

        metric_type: MetricType::Gauge,
        values: vec![MetricValue::from_value(
            con.scard(&*keys::session::LIST_TERMINATED)
                .await
                .unwrap_or_default(),
        )],
    }
}

pub async fn slots_total<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Metric {
    let orchestrators: Vec<String> = con
        .smembers(&*keys::orchestrator::LIST)
        .await
        .unwrap_or_default();

    let mut slot_count = 0.0;
    for orchestrator_id in orchestrators {
        slot_count += con
            .scard::<_, f64>(keys::orchestrator::slots::allocated(&orchestrator_id))
            .await
            .unwrap_or_default();
    }

    Metric {
        name: "webgrid_slots_allocated".to_string(),
        description: "Total number of allocated slots".to_string(),

        metric_type: MetricType::Gauge,
        values: vec![MetricValue::from_value(slot_count)],
    }
}

pub async fn slots_available<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Metric {
    let orchestrators: Vec<String> = con
        .smembers(&*keys::orchestrator::LIST)
        .await
        .unwrap_or_default();

    let mut slot_count = 0.0;
    for orchestrator_id in orchestrators {
        slot_count += con
            .llen::<_, f64>(keys::orchestrator::slots::available(&orchestrator_id))
            .await
            .unwrap_or_default();
    }

    Metric {
        name: "webgrid_slots_available".to_string(),
        description: "Total number of available slots".to_string(),

        metric_type: MetricType::Gauge,
        values: vec![MetricValue::from_value(slot_count)],
    }
}
