-- Search performance indexes
-- Enables fast lookups for category filter, price range, and keyword search

-- Enable pg_trgm extension for trigram indexes (used by ILIKE '%keyword%' queries)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Category filter support (B-tree, used for = 'electronics' queries)
CREATE INDEX IF NOT EXISTS inventory_category_idx ON inventory(category);

-- Price range queries (B-tree, used for WHERE price >= X AND price <= Y)
CREATE INDEX IF NOT EXISTS inventory_price_idx ON inventory(suggested_price_cny);

-- Trigram indexes for title and description keyword search
-- These make ILIKE '%keyword%' use the index instead of full table scan
CREATE INDEX IF NOT EXISTS inventory_title_trgm_idx ON inventory USING gin(title gin_trgm_ops);
CREATE INDEX IF NOT EXISTS inventory_description_trgm_idx ON inventory USING gin(description gin_trgm_ops);

-- documents table: index on id for fast lookups
-- (the documents.id column stores the listing_id value as text)
CREATE INDEX IF NOT EXISTS documents_listing_id_idx ON documents(id);

-- Composite index for common filter combinations (status + category)
CREATE INDEX IF NOT EXISTS inventory_status_category_idx ON inventory(status, category);
