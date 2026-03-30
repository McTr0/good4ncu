ALTER TABLE chat_messages
ADD COLUMN IF NOT EXISTS image_url TEXT,
ADD COLUMN IF NOT EXISTS audio_url TEXT;

CREATE INDEX IF NOT EXISTS idx_chat_messages_image_url
ON chat_messages(image_url)
WHERE image_url IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_chat_messages_audio_url
ON chat_messages(audio_url)
WHERE audio_url IS NOT NULL;
