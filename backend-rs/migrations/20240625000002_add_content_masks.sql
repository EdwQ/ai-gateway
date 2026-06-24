-- Add content_masks table for storing sensitive data detection results
-- Part of the security audit feature

-- ============================================================
-- Content Masks (sensitive data detection hits)
-- ============================================================
CREATE TABLE IF NOT EXISTS content_masks (
    id                SERIAL PRIMARY KEY,
    call_content_id   UUID NOT NULL REFERENCES call_contents(id) ON DELETE CASCADE,
    mask_type         VARCHAR(64) NOT NULL,
    mask_pattern      VARCHAR(256) NOT NULL,
    match_count       INTEGER NOT NULL DEFAULT 1,
    matched_fields    JSONB DEFAULT '[]'::jsonb,
    severity          VARCHAR(16) NOT NULL DEFAULT 'info',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_content_masks_call_content_id ON content_masks(call_content_id);
CREATE INDEX idx_content_masks_mask_type ON content_masks(mask_type);
CREATE INDEX idx_content_masks_severity ON content_masks(severity);
CREATE INDEX idx_content_masks_created_at ON content_masks(created_at);
