-- Add daily_usage_stats table for long-term aggregated statistics
-- Part of the behavior analysis feature

-- ============================================================
-- Daily Usage Stats (long-term aggregation)
-- ============================================================
CREATE TABLE IF NOT EXISTS daily_usage_stats (
    id                SERIAL PRIMARY KEY,
    stat_date         DATE NOT NULL,
    user_id           UUID NOT NULL REFERENCES users(id),
    model             VARCHAR(128) NOT NULL,
    provider          VARCHAR(64) NOT NULL,
    total_calls       INTEGER NOT NULL DEFAULT 0,
    total_input_tokens  BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0,
    total_cost        NUMERIC(12, 6) NOT NULL DEFAULT 0,
    avg_latency_ms    NUMERIC(10, 2) NOT NULL DEFAULT 0,
    error_count       INTEGER NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(stat_date, user_id, model, provider)
);

CREATE INDEX idx_daily_usage_stats_date ON daily_usage_stats(stat_date);
CREATE INDEX idx_daily_usage_stats_user_id ON daily_usage_stats(user_id);
CREATE INDEX idx_daily_usage_stats_model ON daily_usage_stats(model);
CREATE INDEX idx_daily_usage_stats_date_user ON daily_usage_stats(stat_date, user_id);
