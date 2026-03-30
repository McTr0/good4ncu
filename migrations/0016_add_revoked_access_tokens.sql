-- 0016: Persist revoked JWT access token IDs (JTI)
--
-- Provides cross-instance and restart-safe token revocation checks.

CREATE TABLE IF NOT EXISTS revoked_access_tokens (
    jti TEXT PRIMARY KEY,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_revoked_access_tokens_expires_at
    ON revoked_access_tokens (expires_at);
