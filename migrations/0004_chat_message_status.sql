-- Add message editing and status fields to chat_messages
ALTER TABLE chat_messages ADD COLUMN IF NOT EXISTS edited_at TIMESTAMPTZ;
ALTER TABLE chat_messages ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'sent';

-- Add unread_count to chat_connections for unread badge feature
ALTER TABLE chat_connections ADD COLUMN IF NOT EXISTS unread_count INTEGER NOT NULL DEFAULT 0;
