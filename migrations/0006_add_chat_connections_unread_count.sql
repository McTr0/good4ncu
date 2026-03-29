-- Add missing unread_count column to chat_connections table
-- This column was referenced in list_connections query but never existed in the schema
ALTER TABLE chat_connections ADD COLUMN IF NOT EXISTS unread_count INTEGER NOT NULL DEFAULT 0;
