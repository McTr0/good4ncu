-- 0015: Fix chat_messages.conversation_id JOIN performance
--
-- Problem: chat_connections.id is UUID, chat_messages.conversation_id is TEXT.
-- JOINs require cc.id::text cast, defeating B-Tree index usage.
--
-- Approach: Keep conversation_id as TEXT (needed for special IDs like '__agent__',
-- 'global') but fix the JOIN direction: cast the parameter to text instead of
-- casting the indexed column. Add a proper index on conversation_id.
--
-- The real fix: ensure JOINs use cm.conversation_id = $1::text (index-friendly)
-- instead of cc.id::text = cm.conversation_id (cast on indexed column).

-- Step 1: B-Tree index on conversation_id for fast lookups
CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation_id
  ON chat_messages(conversation_id);

-- Step 2: Composite index for the batch-read query pattern
-- (conversation_id + receiver + read_at IS NULL)
CREATE INDEX IF NOT EXISTS idx_chat_messages_unread
  ON chat_messages(conversation_id, receiver)
  WHERE read_at IS NULL;
