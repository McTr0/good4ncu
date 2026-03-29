-- Admin Audit Logs Table
-- Logs all sensitive administrative actions for security and debugging.

CREATE TABLE IF NOT EXISTS admin_audit_logs (
    id TEXT PRIMARY KEY,
    admin_id TEXT NOT NULL,
    action TEXT NOT NULL,       -- "ban_user", "update_role", "takedown_listing", "impersonate"
    target_id TEXT,             -- Affected user_id or listing_id
    old_value TEXT,             -- Previous state if applicable (JSON or string)
    new_value TEXT,             -- New state if applicable (JSON or string)
    memo TEXT,                  -- Optional contextual note
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    -- Constraints (assuming admins table uses TEXT IDs, which users table does)
    FOREIGN KEY (admin_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for fast lookup by admin or target
CREATE INDEX IF NOT EXISTS idx_admin_audit_admin_id ON admin_audit_logs(admin_id);
CREATE INDEX IF NOT EXISTS idx_admin_audit_target_id ON admin_audit_logs(target_id);
CREATE INDEX IF NOT EXISTS idx_admin_audit_action ON admin_audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_admin_audit_created_at ON admin_audit_logs(created_at DESC);
