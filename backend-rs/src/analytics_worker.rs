use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::AppConfig;

pub struct AnalyticsWorker {
    pool: PgPool,
    config: Arc<AppConfig>,
}

impl AnalyticsWorker {
    pub fn new(pool: PgPool, config: Arc<AppConfig>) -> Self {
        Self { pool, config }
    }

    pub async fn start(self) {
        let agg_interval = std::time::Duration::from_secs(3600);

        loop {
            self.run_aggregation().await;
            self.run_cleanup().await;

            tokio::time::sleep(agg_interval).await;
        }
    }

    async fn run_aggregation(&self) {
        let retention_days = self.config.content_capture_config.retention_days as i64;

        // Aggregate un-aggregated call_contents into daily_usage_stats
        // We only aggregate data older than 1 day (to ensure day is complete)
        let cutoff = Utc::now().date_naive() - chrono::Duration::days(1);

        let result = sqlx::query(
            r#"INSERT INTO daily_usage_stats
               (stat_date, user_id, model, provider,
                total_calls, total_input_tokens, total_output_tokens,
                total_cost, avg_latency_ms, error_count)
               SELECT
                   DATE(cc.created_at) as stat_date,
                   cc.user_id,
                   cc.model,
                   cc.provider,
                   COUNT(*)::int as total_calls,
                   COALESCE(SUM(cc.input_tokens), 0)::bigint as total_input_tokens,
                   COALESCE(SUM(cc.output_tokens), 0)::bigint as total_output_tokens,
                   COALESCE(SUM(ul.cost_rmb), 0) as total_cost,
                   COALESCE(AVG(cc.latency_ms)::numeric(10,2), 0) as avg_latency_ms,
                   COUNT(*) FILTER (WHERE ul.is_success = false)::int as error_count
               FROM call_contents cc
               LEFT JOIN usage_logs ul ON ul.request_id = cc.request_id
               WHERE DATE(cc.created_at) <= $1
                 AND cc.created_at >= NOW() - make_interval(days => $2)
               GROUP BY stat_date, cc.user_id, cc.model, cc.provider
               ON CONFLICT (stat_date, user_id, model, provider)
               DO UPDATE SET
                   total_calls = EXCLUDED.total_calls,
                   total_input_tokens = EXCLUDED.total_input_tokens,
                   total_output_tokens = EXCLUDED.total_output_tokens,
                   total_cost = EXCLUDED.total_cost,
                   avg_latency_ms = EXCLUDED.avg_latency_ms,
                   error_count = EXCLUDED.error_count
               "#,
        )
        .bind(cutoff)
        .bind(retention_days)
        .execute(&self.pool)
        .await;

        match result {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    tracing::info!("Aggregated {} rows into daily_usage_stats", r.rows_affected());
                }
            }
            Err(e) => tracing::warn!("Aggregation query failed: {}", e),
        }
    }

    async fn run_cleanup(&self) {
        let retention_days = self.config.content_capture_config.retention_days as i64;

        let result = sqlx::query(
            "DELETE FROM call_contents WHERE expires_at < NOW()",
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    tracing::info!(
                        "Cleaned up {} expired call_contents records (retention: {} days)",
                        r.rows_affected(),
                        retention_days
                    );
                }
            }
            Err(e) => tracing::warn!("Cleanup query failed: {}", e),
        }
    }
}
