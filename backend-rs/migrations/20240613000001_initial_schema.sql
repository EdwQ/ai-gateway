-- Initial schema for AI Gateway
-- Ported from SQLAlchemy models in backend/app/models/

-- ============================================================
-- Users
-- ============================================================
CREATE TABLE IF NOT EXISTS users (
    id              UUID PRIMARY KEY,
    union_id        VARCHAR(256) NOT NULL UNIQUE,
    user_id         VARCHAR(128),
    name            VARCHAR(256) NOT NULL,
    email           VARCHAR(256),
    avatar          TEXT,
    department_id   VARCHAR(64),
    department_name VARCHAR(256),
    title           VARCHAR(128),
    role            VARCHAR(32) NOT NULL DEFAULT 'employee',
    is_active       BOOLEAN NOT NULL DEFAULT true,
    quota_balance   NUMERIC(12, 4) NOT NULL DEFAULT 50.0000,
    quota_used      NUMERIC(12, 4) NOT NULL DEFAULT 0.0000,
    last_login_at   TIMESTAMPTZ,
    allowed_models  JSONB DEFAULT '[]'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_union_id ON users(union_id);
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_users_is_active ON users(is_active);

-- ============================================================
-- API Tokens
-- ============================================================
CREATE TABLE IF NOT EXISTS api_tokens (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   VARCHAR(128) NOT NULL UNIQUE,
    token_prefix VARCHAR(32) NOT NULL,
    name         VARCHAR(128) NOT NULL DEFAULT '',
    is_active    BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMPTZ,
    expires_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_tokens_user_id ON api_tokens(user_id);
CREATE INDEX idx_api_tokens_token_hash ON api_tokens(token_hash);

-- ============================================================
-- Providers
-- ============================================================
CREATE TABLE IF NOT EXISTS providers (
    id                UUID PRIMARY KEY,
    name              VARCHAR(64) NOT NULL UNIQUE,
    display_name      VARCHAR(128) NOT NULL,
    base_url          VARCHAR(512) NOT NULL,
    api_key_encrypted TEXT NOT NULL,
    models            JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_active         BOOLEAN NOT NULL DEFAULT true,
    priority          INTEGER NOT NULL DEFAULT 100,
    health_status     VARCHAR(32) NOT NULL DEFAULT 'unknown',
    rate_limit_qps    INTEGER NOT NULL DEFAULT 60,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_providers_name ON providers(name);

-- ============================================================
-- Provider Keys (multi-key support with failover)
-- ============================================================
CREATE TABLE IF NOT EXISTS provider_keys (
    id              UUID PRIMARY KEY,
    provider_id     UUID NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    key_encrypted   TEXT NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT true,
    weight          INTEGER NOT NULL DEFAULT 1,
    fail_count      INTEGER NOT NULL DEFAULT 0,
    max_fail_count  INTEGER NOT NULL DEFAULT 3,
    last_success_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_provider_keys_provider_id ON provider_keys(provider_id);

-- ============================================================
-- Usage Logs
-- ============================================================
CREATE TABLE IF NOT EXISTS usage_logs (
    id                SERIAL PRIMARY KEY,
    user_id           UUID NOT NULL REFERENCES users(id),
    token_id          UUID,
    model             VARCHAR(128) NOT NULL,
    provider          VARCHAR(64) NOT NULL,
    prompt_tokens     INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens      INTEGER NOT NULL DEFAULT 0,
    cost_rmb          NUMERIC(12, 6) NOT NULL DEFAULT 0,
    duration_ms       INTEGER NOT NULL DEFAULT 0,
    is_stream         BOOLEAN NOT NULL DEFAULT false,
    is_success        BOOLEAN NOT NULL DEFAULT true,
    status_code       INTEGER NOT NULL DEFAULT 200,
    error_message     TEXT,
    ip_address        VARCHAR(45),
    request_id        VARCHAR(64) UNIQUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_usage_logs_user_id ON usage_logs(user_id);
CREATE INDEX idx_usage_logs_model ON usage_logs(model);
CREATE INDEX idx_usage_logs_created_at ON usage_logs(created_at);
CREATE INDEX idx_usage_logs_request_id ON usage_logs(request_id);

-- ============================================================
-- Model Aliases
-- ============================================================
CREATE TABLE IF NOT EXISTS model_aliases (
    id           UUID PRIMARY KEY,
    alias_name   VARCHAR(128) NOT NULL UNIQUE,
    target_model VARCHAR(256) NOT NULL,
    description  VARCHAR(512),
    is_active    BOOLEAN NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_model_aliases_alias_name ON model_aliases(alias_name);

-- ============================================================
-- Audit Logs
-- ============================================================
CREATE TABLE IF NOT EXISTS audit_logs (
    id            SERIAL PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users(id),
    action        VARCHAR(64) NOT NULL,
    resource_type VARCHAR(64) NOT NULL,
    resource_id   VARCHAR(128),
    details       JSONB,
    ip_address    VARCHAR(45),
    user_agent    VARCHAR(512),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);

-- ============================================================
-- Prompt Audits (optional, content-sensitive)
-- ============================================================
CREATE TABLE IF NOT EXISTS prompt_audits (
    id                  SERIAL PRIMARY KEY,
    usage_log_id        INTEGER NOT NULL UNIQUE REFERENCES usage_logs(id),
    save_mode           VARCHAR(32) NOT NULL,
    prompt_content      TEXT,
    prompt_summary      VARCHAR(512),
    completion_content  TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_prompt_audits_usage_log_id ON prompt_audits(usage_log_id);

-- ============================================================
-- Departments (cached from DingTalk)
-- ============================================================
CREATE TABLE IF NOT EXISTS departments (
    id         VARCHAR(64) PRIMARY KEY,
    name       VARCHAR(256) NOT NULL,
    parent_id  VARCHAR(64),
    order_num  INTEGER,
    is_active  BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
