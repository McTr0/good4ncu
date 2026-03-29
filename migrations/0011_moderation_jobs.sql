-- Content moderation jobs table and per-resource moderation status columns.
-- Image moderation is async: jobs are submitted on content creation and processed by
-- the moderation worker, which calls the external image moderation API.

-- Moderation job record for async image moderation.
CREATE TABLE IF NOT EXISTS moderation_jobs (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    resource_type TEXT NOT NULL,  -- 'listing_image', 'chat_image', 'avatar'
    resource_id TEXT NOT NULL,    -- FK to inventory.id, chat_messages.id, or users.id
    image_url TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'failed')),
    reject_reason TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_moderation_jobs_status_created
    ON moderation_jobs(status, created_at);

-- Per-resource moderation status on inventory (for listing images).
ALTER TABLE inventory
    ADD COLUMN IF NOT EXISTS images_moderation_status TEXT NOT NULL DEFAULT 'approved';

-- Per-message moderation status on chat_messages.
ALTER TABLE chat_messages
    ADD COLUMN IF NOT EXISTS moderation_status TEXT NOT NULL DEFAULT 'approved';

-- Per-user avatar moderation status.
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS avatar_moderation_status TEXT NOT NULL DEFAULT 'pending';
