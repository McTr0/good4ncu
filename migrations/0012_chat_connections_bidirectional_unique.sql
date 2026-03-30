-- De-duplicate existing connections in case of bidirectional duplicates
DELETE FROM chat_connections a
USING chat_connections b
WHERE 
    LEAST(a.requester_id, a.receiver_id) = LEAST(b.requester_id, b.receiver_id) 
    AND GREATEST(a.requester_id, a.receiver_id) = GREATEST(b.requester_id, b.receiver_id)
    AND a.created_at > b.created_at;

-- Drop the single-directional unique constraint
ALTER TABLE chat_connections DROP CONSTRAINT IF EXISTS chat_connections_requester_id_receiver_id_key;

-- Prevent users from connecting with themselves
ALTER TABLE chat_connections ADD CONSTRAINT chat_connections_no_self_loop CHECK (requester_id != receiver_id);

-- Add the bidirectional unique index
CREATE UNIQUE INDEX IF NOT EXISTS chat_connections_bidirectional_idx 
ON chat_connections (
    LEAST(requester_id, receiver_id), 
    GREATEST(requester_id, receiver_id)
);
