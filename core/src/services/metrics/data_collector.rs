use crate::{Metric, MetricType, MetricValue};
use redis::{aio::ConnectionManager, AsyncCommands};
use shared::metrics::SESSION_STARTUP_HISTOGRAM_BUCKETS;

static HTTP_METHODS: [&str; 9] = [
    "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "CONNECT", "PATCH", "TRACE",
];
static LOG_LEVELS: [&str; 3] = ["INFO", "WARN", "FAIL"];

pub async fn traffic_value(con: &mut ConnectionManager, direction: &str) -> MetricValue {
    let bytes: f64 = con
        .hget("metrics:http:net.bytes.total", direction)
        .await
        .unwrap_or_default();

    MetricValue::from_value(bytes).with_label("direction", direction)
}

pub async fn proxy_traffic(con: &ConnectionManager) -> Metric {
    let mut con = con.clone();

    Metric {
        name: "webgrid_proxy_http_net_bytes_total".to_string(),
        description: "Bytes (body only) transferred through all proxy instances".to_string(),

        metric_type: MetricType::Counter,
        values: vec![
            traffic_value(&mut con, "in").await,
            traffic_value(&mut con, "out").await,
        ],
    }
}

pub async fn proxy_requests(con: &ConnectionManager) -> Metric {
    let mut con = con.clone();
    let mut metric = Metric {
        name: "webgrid_proxy_http_requests_total".to_string(),
        description: "Requests processed by all proxy instances".to_string(),

        metric_type: MetricType::Counter,
        values: Vec::new(),
    };

    for method in HTTP_METHODS.iter() {
        if let Ok(status_codes) = con
            .hgetall::<_, Vec<(String, f64)>>(format!("metrics:http:requestsTotal:{}", method))
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

pub async fn session_log(con: &ConnectionManager) -> Metric {
    let mut con = con.clone();
    let mut metric = Metric {
        name: "webgrid_session_status_codes_total".to_string(),
        description: "Log codes for sessions".to_string(),

        metric_type: MetricType::Counter,
        values: Vec::new(),
    };

    for log_level in LOG_LEVELS.iter() {
        if let Ok(log_codes) = con
            .hgetall::<_, Vec<(String, f64)>>(format!("metrics:sessions:log:{}", log_level))
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

pub async fn session_startup_duration(con: &ConnectionManager) -> Metric {
    let mut con = con.clone();
    let mut metric = Metric {
        name: "session_startup_duration_seconds".to_string(),
        description: "Total startup duration of a session including queue and scheduling"
            .to_string(),

        metric_type: MetricType::Histogram,
        values: Vec::new(),
    };

    for bucket in SESSION_STARTUP_HISTOGRAM_BUCKETS.iter() {
        let bucket_name = bucket.to_string();
        let bucket_value = con
            .hget("metrics:sessions:startup.histogram:buckets", &bucket_name)
            .await
            .unwrap_or_default();

        metric.values.push(
            MetricValue::from_value(bucket_value)
                .with_label("le", &bucket_name)
                .with_name_postfix("_bucket"),
        );
    }

    let infinity_bucket_value = con
        .hget("metrics:sessions:startup.histogram:buckets", "+Inf")
        .await
        .unwrap_or_default();
    metric.values.push(
        MetricValue::from_value(infinity_bucket_value)
            .with_label("le", "+Inf")
            .with_name_postfix("_bucket"),
    );

    let sum_value = con
        .get("metrics:sessions:startup.histogram:sum")
        .await
        .unwrap_or_default();
    metric
        .values
        .push(MetricValue::from_value(sum_value).with_name_postfix("_sum"));

    let sum_value = con
        .get("metrics:sessions:startup.histogram:count")
        .await
        .unwrap_or_default();
    metric
        .values
        .push(MetricValue::from_value(sum_value).with_name_postfix("_count"));

    metric
}

pub async fn sessions_active(con: &ConnectionManager) -> Metric {
    let mut con = con.clone();

    Metric {
        name: "sessions_active".to_string(),
        description: "Current number of active sessions (including queued and pending)".to_string(),

        metric_type: MetricType::Gauge,
        values: vec![MetricValue::from_value(
            con.scard("sessions.active").await.unwrap_or_default(),
        )],
    }
}
