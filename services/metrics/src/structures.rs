use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

impl fmt::Display for MetricType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

pub struct MetricValue {
    pub name_postfix: Option<String>,
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp: Option<DateTime<Utc>>,
}

impl MetricValue {
    pub fn from_value(value: f64) -> Self {
        Self {
            value,
            labels: HashMap::new(),
            name_postfix: None,
            timestamp: None,
        }
    }

    pub fn with_name_postfix(mut self, postfix: &str) -> Self {
        self.name_postfix = Some(postfix.to_string());
        self
    }

    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }
}

impl fmt::Display for MetricValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.name_postfix.clone().unwrap_or_else(|| "".to_string())
        )?;

        if !self.labels.is_empty() {
            let mut formatted_labels =
                self.labels.iter().fold(String::new(), |res, (key, value)| {
                    res + &format!("{}=\"{}\"", key, value) + ","
                });

            // Remove trailing comma
            formatted_labels.pop();

            write!(f, "{{{}}}", formatted_labels)?;
        }

        if let Some(timestamp) = self.timestamp {
            write!(f, " {} {}", self.value, timestamp.timestamp())
        } else {
            write!(f, " {}", self.value)
        }
    }
}

pub struct Metric {
    pub description: String,
    pub metric_type: MetricType,

    pub name: String,
    pub values: Vec<MetricValue>,
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted_description = format!("# HELP {} {}", self.name, self.description);
        let formatted_metric_type = format!("# TYPE {} {}", self.name, self.metric_type);
        let formatted_values = self.values.iter().fold(String::new(), |res, value| {
            res + &format!("{}{}", self.name, value) + "\n"
        });

        write!(
            f,
            "{}\n{}\n{}",
            formatted_description, formatted_metric_type, formatted_values
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_without_labels() {
        let value = MetricValue {
            name_postfix: None,
            labels: HashMap::new(),
            timestamp: None,
            value: 10.0,
        };

        let actual = format!("{}", value);
        let expected = " 10";

        assert_eq!(actual, expected);
    }

    #[test]
    fn value_with_label() {
        let mut value = MetricValue {
            name_postfix: None,
            labels: HashMap::new(),
            timestamp: None,
            value: 10.0,
        };

        value.labels.insert("test".to_string(), "label".to_string());

        let actual = format!("{}", value);
        let expected = "{test=\"label\"} 10";

        assert_eq!(actual, expected);
    }

    #[test]
    fn value_with_multiple_labels() {
        let mut value = MetricValue {
            name_postfix: None,
            labels: HashMap::new(),
            timestamp: None,
            value: 10.0,
        };

        value.labels.insert("test".to_string(), "label".to_string());
        value
            .labels
            .insert("other".to_string(), "labelvalue".to_string());

        let actual = format!("{}", value);
        let expected = "{test=\"label\",other=\"labelvalue\"} 10";
        let or_expected = "{other=\"labelvalue\",test=\"label\"} 10";

        let is_expected = actual == expected || actual == or_expected;
        assert_eq!(is_expected, true);
    }

    #[test]
    fn metric_without_values() {
        let metric = Metric {
            description: "TestDescription".to_string(),
            metric_type: MetricType::Counter,

            name: "test_metric".to_string(),
            values: Vec::new(),
        };

        let actual = format!("{}", metric);
        let expected = "# HELP test_metric TestDescription\n# TYPE test_metric counter\n";

        assert_eq!(actual, expected);
    }

    #[test]
    fn metric_with_value() {
        let mut metric = Metric {
            description: "TestDescription".to_string(),
            metric_type: MetricType::Counter,

            name: "test_metric".to_string(),
            values: Vec::new(),
        };

        let value = MetricValue {
            name_postfix: None,
            labels: HashMap::new(),
            timestamp: None,
            value: 10.0,
        };

        metric.values.push(value);

        let actual = format!("{}", metric);
        let expected =
            "# HELP test_metric TestDescription\n# TYPE test_metric counter\ntest_metric 10\n";

        assert_eq!(actual, expected);
    }

    #[test]
    fn metric_with_multiple_values() {
        let mut metric = Metric {
            description: "TestDescription".to_string(),
            metric_type: MetricType::Counter,

            name: "test_metric".to_string(),
            values: Vec::new(),
        };

        let value = MetricValue {
            name_postfix: None,
            labels: HashMap::new(),
            timestamp: None,
            value: 10.0,
        };

        let other_value = MetricValue {
            name_postfix: None,
            labels: HashMap::new(),
            timestamp: None,
            value: 15.0,
        };

        metric.values.push(value);
        metric.values.push(other_value);

        let actual = format!("{}", metric);
        let expected = "# HELP test_metric TestDescription\n# TYPE test_metric counter\ntest_metric 10\ntest_metric 15\n";

        assert_eq!(actual, expected);
    }

    #[test]
    fn metric_with_multiple_values_and_labels() {
        let mut metric = Metric {
            description: "TestDescription".to_string(),
            metric_type: MetricType::Counter,

            name: "test_metric".to_string(),
            values: Vec::new(),
        };

        let mut value = MetricValue {
            name_postfix: None,
            labels: HashMap::new(),
            timestamp: None,
            value: 10.0,
        };

        let mut other_value = MetricValue {
            name_postfix: None,
            labels: HashMap::new(),
            timestamp: None,
            value: 15.0,
        };

        value
            .labels
            .insert("label1".to_string(), "value1".to_string());
        other_value
            .labels
            .insert("label2".to_string(), "value2".to_string());

        metric.values.push(value);
        metric.values.push(other_value);

        let actual = format!("{}", metric);
        let expected = "# HELP test_metric TestDescription\n# TYPE test_metric counter\ntest_metric{label1=\"value1\"} 10\ntest_metric{label2=\"value2\"} 15\n";

        assert_eq!(actual, expected);
    }
}
