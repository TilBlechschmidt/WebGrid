# Prometheus metrics

## Actual metrics
### proxy_http
- proxy_http_requests_total{method="...",code="..."} counter ✅
- proxy_http_net_bytes_total{direction="in|out"} counter ✅

### sessions
- sessions_active gauge
- session_total{stage="queued|pending|alive|terminated"} counter
- session_duration_seconds_total{stage="queued|pending|alive"} counter
- session_status_codes_total{level="...",code="<any log code>"} counter ✅
- session_startup_duration_seconds histogram ✅
	- session_startup_duration_seconds_bucket{le="2"}
	- session_startup_duration_seconds_bucket{le="4"}
	- session_startup_duration_seconds_bucket{le="8"}
	- session_startup_duration_seconds_bucket{le="16"}
	- session_startup_duration_seconds_bucket{le="32"}
	- session_startup_duration_seconds_bucket{le="64"}
	- session_startup_duration_seconds_bucket{le="+Inf"}
	- session_startup_duration_seconds_sum
	- session_startup_duration_seconds_count

### slots
- slots_reclaimed_total{source="dead|orphaned"} counter
- slots_total{<browser>=true} gauge
- slots_available{<browser>=true} gauge

## High level
### Counters
- Served session HTTP requests .
	- Methods .
	- Status codes .
- Total time spent .
	- Queued .
	- Pending .
	- Alive .
- Total sessions .
	- Queued .
	- Pending .
	- Alive .
	- Terminated .
- Total incoming control bytes @ proxy .
- Total outgoing control bytes @ proxy .
- Reclaimed slots (orchestratorID) .
	- Dead .
	- Orphaned .
- Session status codes .

### Gauges
- Slots (chrome=true, firefox=true, ...) .
	- Total .
	- Available .

## Histograms
- Session startup time .
- Session duration