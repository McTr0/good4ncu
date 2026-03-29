-- Add email column to users table for @ncu.edu.cn domain authentication
ALTER TABLE users ADD COLUMN email TEXT UNIQUE;

-- Index for fast email lookups during login/registration
CREATE UNIQUE INDEX IF NOT EXISTS users_email_idx ON users(email) WHERE email IS NOT NULL;
