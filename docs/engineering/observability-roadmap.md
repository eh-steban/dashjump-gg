# Observability Roadmap

Future phases for logging, metrics, and tracing infrastructure.

## Phase 2: Structured Logging
- JSON format for all services
- Consistent field names: `timestamp`, `level`, `service`, `correlation_id`, `message`
- Environment-based log level configuration (`LOG_LEVEL` env var)

## Phase 3: Centralized Logging
- ELK stack (Elasticsearch, Logstash, Kibana) or similar
- Log aggregation from all services
- Searchable, filterable logs
- Log retention policies enforced

## Phase 4: Metrics & Tracing
- Prometheus metrics (request latency, error rates, queue depths)
- Distributed tracing (OpenTelemetry)
- Error tracking service (Sentry)
- Service health endpoints

## Phase 5: Alerting & Dashboards
- Grafana dashboards for key metrics
- PagerDuty/Slack alerting on error thresholds
- SLO/SLA monitoring
- Capacity planning data

## Data Retention Policy (Future)

| Data Type | Retention | Rationale |
|-----------|-----------|-----------|
| Debug logs | 7 days | Short-term troubleshooting |
| Info logs | 30 days | Operational review window |
| Error logs | 90 days | Incident analysis period |
| Metrics | 1 year | Trend analysis, capacity planning |
| Traces | 7 days | Request debugging |
