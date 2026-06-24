-- Add call_contents table for storing AI API request/response content
-- Part of the behavior analysis feature

-- ============================================================
-- Call Contents (short-term full-text storage)
-- ============================================================
CREATE TABLE IF NOT EXISTS call_contents (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id),
    token_id          UUID REFERENCES api_tokens(id),
    request_id        VARCHAR(64) REFERENCES usage_logs(request_id),
    model             VARCHAR(128) NOT NULL,
    provider          VARCHAR(64) NOT NULL,
    request_content   JSONB NOT NULL,
    response_content  JSONB,
    file_metadata     JSONB DEFAULT '[]'::jsonb,
    input_tokens      INTEGER NOT NULL DEFAULT 0,
    output_tokens     INTEGER NOT NULL DEFAULT 0,
    latency_ms        INTEGER NOT NULL DEFAULT 0,
    is_stream         BOOLEAN NOT NULL DEFAULT false,
    ip_address        VARCHAR(45),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at        TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '30 days')
);

-- Primary query patterns: by user, by model, by time range
CREATE INDEX idx_call_contents_user_id ON call_contents(user_id);
CREATE INDEX idx_call_contents_model ON call_contents(model);
CREATE INDEX idx_call_contents_created_at ON call_contents(created_at);
CREATE INDEX idx_call_contents_expires_at ON call_contents(expires_at);

-- Composite index for common dashboard queries
CREATE INDEX idx_call_contents_user_time ON call_contents(user_id, created_at DESC);
CREATE INDEX idx_call_contents_model_time ON call_contents(model, created_at DESC);

-- Full-text search support (requires pg_trgm extension)
-- The extension should be created by the migration runner if not exists
CREATE INDEX idx_call_contents_request_gin ON call_contents USING GIN (request_content jsonb_path_ops);
CREATE INDEX idx_call_contents_response_gin ON call_contents USING GIN (response_content jsonb_path_ops);
