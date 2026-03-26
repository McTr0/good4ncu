-- V1__init.sql: Initial database schema for good4ncu
-- Created for P8-Backend Data Persistence Evolution

-- Enable pgvector extension (creates the vector type and operators)
CREATE EXTENSION IF NOT EXISTS vector;

-- ============================================================================
-- Users table
-- ============================================================================
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================================
-- Inventory (listings) table
-- ============================================================================
CREATE TABLE IF NOT EXISTS inventory (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    category TEXT NOT NULL,
    brand TEXT NOT NULL,
    condition_score INTEGER NOT NULL CHECK (condition_score >= 1 AND condition_score <= 10),
    suggested_price_cny BIGINT NOT NULL CHECK (suggested_price_cny >= 0),
    defects TEXT NOT NULL,
    description TEXT,
    owner_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(owner_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ============================================================================
-- Orders table
-- ============================================================================
CREATE TABLE IF NOT EXISTS orders (
    id TEXT PRIMARY KEY,
    listing_id TEXT NOT NULL,
    buyer_id TEXT NOT NULL,
    seller_id TEXT NOT NULL,
    final_price BIGINT NOT NULL CHECK (final_price >= 0),
    status TEXT NOT NULL,
    cancellation_reason TEXT,
    paid_at TIMESTAMPTZ,
    shipped_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(listing_id) REFERENCES inventory(id) ON DELETE CASCADE,
    FOREIGN KEY(buyer_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(seller_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ============================================================================
-- Chat messages table
-- ============================================================================
CREATE TABLE IF NOT EXISTS chat_messages (
    id BIGSERIAL PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    listing_id TEXT NOT NULL,
    sender TEXT NOT NULL,
    receiver TEXT,
    is_agent BOOLEAN NOT NULL DEFAULT FALSE,
    content TEXT NOT NULL,
    image_data TEXT,
    audio_data TEXT,
    read_at TIMESTAMPTZ,
    read_by TEXT,
    timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(sender) REFERENCES users(id) ON DELETE CASCADE
);

-- ============================================================================
-- Watchlist table (user favorites)
-- ============================================================================
CREATE TABLE IF NOT EXISTS watchlist (
    user_id TEXT NOT NULL,
    listing_id TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, listing_id),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(listing_id) REFERENCES inventory(id) ON DELETE CASCADE
);

-- ============================================================================
-- Documents table (pgvector RAG embeddings)
-- Stores listing content as vectors for semantic search.
-- VECTOR_DIM defaults to 768 to match the default embedding model.
-- ============================================================================
CREATE TABLE IF NOT EXISTS documents (
    id TEXT NOT NULL,
    document JSONB NOT NULL,
    embedded_text TEXT NOT NULL,
    embedding vector(768)
);

-- ============================================================================
-- Notifications table
-- Seller receives an in-app notification when a buyer places an order.
-- ============================================================================
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    related_order_id TEXT,
    related_listing_id TEXT,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ============================================================================
-- HITL requests table
-- Stores pending seller approval requests for negotiation.
-- The seller responds via PATCH /api/negotiations/{id} with approve/reject/counter.
-- The marketplace agent waits on this record being resolved before proceeding.
-- ============================================================================
CREATE TABLE IF NOT EXISTS hitl_requests (
    id TEXT PRIMARY KEY,
    listing_id TEXT NOT NULL,
    buyer_id TEXT NOT NULL,
    seller_id TEXT NOT NULL,
    proposed_price BIGINT NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    -- pending: awaiting seller response
    -- approved: deal accepted
    -- rejected: seller declined
    -- countered: seller counter-offered with different price
    counter_price BIGINT,
    buyer_action TEXT,
    -- accepted: buyer accepted seller's counter --> triggers DealReached
    -- rejected: buyer declined seller's counter --> final rejection
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    resolved_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    -- computed at insert time: created_at + INTERVAL '48 hours'
    FOREIGN KEY(listing_id) REFERENCES inventory(id) ON DELETE CASCADE,
    FOREIGN KEY(buyer_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(seller_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ============================================================================
-- Chat connections table (user-to-user direct chat with connection handshake)
-- Implements a three-way handshake for establishing chat connections:
-- 1. Requester sends POST /api/chat/connect/request --> status=pending
-- 2. Receiver accepts via POST /api/chat/connect/accept --> status=connected
--    (or rejects via POST /api/chat/connect/reject --> status=rejected)
-- 3. Once connected, messages can be exchanged via
--    POST /api/chat/conversations/{id}/messages
-- ============================================================================
CREATE TABLE IF NOT EXISTS chat_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    requester_id TEXT NOT NULL REFERENCES users(id),
    receiver_id TEXT NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending',
    established_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(requester_id, receiver_id)
);

-- ============================================================================
-- Indexes
-- ============================================================================

-- Chat indexes
CREATE INDEX IF NOT EXISTS idx_chat_conversation ON chat_messages(conversation_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_chat_sender ON chat_messages(sender);
CREATE INDEX IF NOT EXISTS idx_chat_connections_requester ON chat_connections(requester_id);
CREATE INDEX IF NOT EXISTS idx_chat_connections_receiver ON chat_connections(receiver_id);

-- Order indexes for efficient order history queries
CREATE INDEX IF NOT EXISTS idx_orders_buyer ON orders(buyer_id);
CREATE INDEX IF NOT EXISTS idx_orders_seller ON orders(seller_id);
CREATE INDEX IF NOT EXISTS idx_orders_listing ON orders(listing_id);

-- Watchlist index
CREATE INDEX IF NOT EXISTS idx_watchlist_user ON watchlist(user_id);

-- Notifications index
CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id, is_read, created_at);

-- HITL requests index for seller's pending approval requests
CREATE INDEX IF NOT EXISTS idx_hitl_seller_status ON hitl_requests(seller_id, status);

-- HNSW index on document embeddings for fast cosine similarity queries
CREATE INDEX IF NOT EXISTS document_embeddings_idx ON documents USING hnsw(embedding vector_cosine_ops);
