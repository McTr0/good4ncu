CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS new_id UUID DEFAULT gen_random_uuid();

UPDATE users
SET new_id = gen_random_uuid()
WHERE new_id IS NULL;

ALTER TABLE users
    ALTER COLUMN new_id SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_new_id ON users(new_id);

ALTER TABLE inventory
    ADD COLUMN IF NOT EXISTS new_id UUID DEFAULT gen_random_uuid(),
    ADD COLUMN IF NOT EXISTS new_owner_id UUID;

UPDATE inventory AS i
SET new_id = COALESCE(i.new_id, gen_random_uuid()),
    new_owner_id = u.new_id
FROM users AS u
WHERE u.id = i.owner_id;

ALTER TABLE inventory
    ALTER COLUMN new_id SET NOT NULL,
    ALTER COLUMN new_owner_id SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_inventory_new_id ON inventory(new_id);
CREATE INDEX IF NOT EXISTS idx_inventory_new_owner_id ON inventory(new_owner_id);

ALTER TABLE inventory
    ADD CONSTRAINT inventory_new_owner_id_fkey
    FOREIGN KEY (new_owner_id)
    REFERENCES users(new_id)
    ON UPDATE CASCADE
    ON DELETE CASCADE
    NOT VALID;

ALTER TABLE inventory
    VALIDATE CONSTRAINT inventory_new_owner_id_fkey;

ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS new_id UUID DEFAULT gen_random_uuid(),
    ADD COLUMN IF NOT EXISTS new_listing_id UUID,
    ADD COLUMN IF NOT EXISTS new_buyer_id UUID,
    ADD COLUMN IF NOT EXISTS new_seller_id UUID;

UPDATE orders AS o
SET new_id = COALESCE(o.new_id, gen_random_uuid()),
    new_listing_id = i.new_id,
    new_buyer_id = buyer.new_id,
    new_seller_id = seller.new_id
FROM inventory AS i,
     users AS buyer,
     users AS seller
WHERE i.id = o.listing_id
  AND buyer.id = o.buyer_id
  AND seller.id = o.seller_id;

ALTER TABLE orders
    ALTER COLUMN new_id SET NOT NULL,
    ALTER COLUMN new_listing_id SET NOT NULL,
    ALTER COLUMN new_buyer_id SET NOT NULL,
    ALTER COLUMN new_seller_id SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_new_id ON orders(new_id);
CREATE INDEX IF NOT EXISTS idx_orders_new_listing_id ON orders(new_listing_id);
CREATE INDEX IF NOT EXISTS idx_orders_new_buyer_id ON orders(new_buyer_id);
CREATE INDEX IF NOT EXISTS idx_orders_new_seller_id ON orders(new_seller_id);

ALTER TABLE orders
    ADD CONSTRAINT orders_new_listing_id_fkey
    FOREIGN KEY (new_listing_id)
    REFERENCES inventory(new_id)
    ON UPDATE CASCADE
    ON DELETE CASCADE
    NOT VALID,
    ADD CONSTRAINT orders_new_buyer_id_fkey
    FOREIGN KEY (new_buyer_id)
    REFERENCES users(new_id)
    ON UPDATE CASCADE
    ON DELETE CASCADE
    NOT VALID,
    ADD CONSTRAINT orders_new_seller_id_fkey
    FOREIGN KEY (new_seller_id)
    REFERENCES users(new_id)
    ON UPDATE CASCADE
    ON DELETE CASCADE
    NOT VALID;

ALTER TABLE orders
    VALIDATE CONSTRAINT orders_new_listing_id_fkey;

ALTER TABLE orders
    VALIDATE CONSTRAINT orders_new_buyer_id_fkey;

ALTER TABLE orders
    VALIDATE CONSTRAINT orders_new_seller_id_fkey;

CREATE OR REPLACE FUNCTION sync_users_uuid_shadow()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.new_id IS NULL THEN
        IF TG_OP = 'UPDATE' THEN
            NEW.new_id := OLD.new_id;
        ELSE
            NEW.new_id := gen_random_uuid();
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION sync_inventory_uuid_shadow()
RETURNS TRIGGER AS $$
DECLARE
    expected_owner_uuid UUID;
BEGIN
    IF NEW.new_id IS NULL THEN
        IF TG_OP = 'UPDATE' THEN
            NEW.new_id := OLD.new_id;
        ELSE
            NEW.new_id := gen_random_uuid();
        END IF;
    END IF;

    SELECT new_id
    INTO expected_owner_uuid
    FROM users
    WHERE id = NEW.owner_id;

    IF expected_owner_uuid IS NULL THEN
        RAISE EXCEPTION 'inventory owner_id % has no matching shadow UUID', NEW.owner_id
            USING ERRCODE = '23503';
    END IF;

    IF NEW.new_owner_id IS NULL
       OR (TG_OP = 'UPDATE'
           AND NEW.owner_id IS DISTINCT FROM OLD.owner_id
           AND NEW.new_owner_id = OLD.new_owner_id) THEN
        NEW.new_owner_id := expected_owner_uuid;
    ELSIF NEW.new_owner_id IS DISTINCT FROM expected_owner_uuid THEN
        RAISE EXCEPTION 'inventory new_owner_id mismatch for owner_id %', NEW.owner_id
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION sync_orders_uuid_shadow()
RETURNS TRIGGER AS $$
DECLARE
    expected_listing_uuid UUID;
    expected_buyer_uuid UUID;
    expected_seller_uuid UUID;
BEGIN
    IF NEW.new_id IS NULL THEN
        IF TG_OP = 'UPDATE' THEN
            NEW.new_id := OLD.new_id;
        ELSE
            NEW.new_id := gen_random_uuid();
        END IF;
    END IF;

    SELECT new_id
    INTO expected_listing_uuid
    FROM inventory
    WHERE id = NEW.listing_id;

    SELECT new_id
    INTO expected_buyer_uuid
    FROM users
    WHERE id = NEW.buyer_id;

    SELECT new_id
    INTO expected_seller_uuid
    FROM users
    WHERE id = NEW.seller_id;

    IF expected_listing_uuid IS NULL THEN
        RAISE EXCEPTION 'orders listing_id % has no matching shadow UUID', NEW.listing_id
            USING ERRCODE = '23503';
    END IF;

    IF expected_buyer_uuid IS NULL THEN
        RAISE EXCEPTION 'orders buyer_id % has no matching shadow UUID', NEW.buyer_id
            USING ERRCODE = '23503';
    END IF;

    IF expected_seller_uuid IS NULL THEN
        RAISE EXCEPTION 'orders seller_id % has no matching shadow UUID', NEW.seller_id
            USING ERRCODE = '23503';
    END IF;

    IF NEW.new_listing_id IS NULL
       OR (TG_OP = 'UPDATE'
           AND NEW.listing_id IS DISTINCT FROM OLD.listing_id
           AND NEW.new_listing_id = OLD.new_listing_id) THEN
        NEW.new_listing_id := expected_listing_uuid;
    ELSIF NEW.new_listing_id IS DISTINCT FROM expected_listing_uuid THEN
        RAISE EXCEPTION 'orders new_listing_id mismatch for listing_id %', NEW.listing_id
            USING ERRCODE = '23514';
    END IF;

    IF NEW.new_buyer_id IS NULL
       OR (TG_OP = 'UPDATE'
           AND NEW.buyer_id IS DISTINCT FROM OLD.buyer_id
           AND NEW.new_buyer_id = OLD.new_buyer_id) THEN
        NEW.new_buyer_id := expected_buyer_uuid;
    ELSIF NEW.new_buyer_id IS DISTINCT FROM expected_buyer_uuid THEN
        RAISE EXCEPTION 'orders new_buyer_id mismatch for buyer_id %', NEW.buyer_id
            USING ERRCODE = '23514';
    END IF;

    IF NEW.new_seller_id IS NULL
       OR (TG_OP = 'UPDATE'
           AND NEW.seller_id IS DISTINCT FROM OLD.seller_id
           AND NEW.new_seller_id = OLD.new_seller_id) THEN
        NEW.new_seller_id := expected_seller_uuid;
    ELSIF NEW.new_seller_id IS DISTINCT FROM expected_seller_uuid THEN
        RAISE EXCEPTION 'orders new_seller_id mismatch for seller_id %', NEW.seller_id
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_sync_users_uuid_shadow ON users;
CREATE TRIGGER trg_sync_users_uuid_shadow
    BEFORE INSERT OR UPDATE OF id, new_id
    ON users
    FOR EACH ROW
    EXECUTE FUNCTION sync_users_uuid_shadow();

DROP TRIGGER IF EXISTS trg_sync_inventory_uuid_shadow ON inventory;
CREATE TRIGGER trg_sync_inventory_uuid_shadow
    BEFORE INSERT OR UPDATE OF id, new_id, owner_id, new_owner_id
    ON inventory
    FOR EACH ROW
    EXECUTE FUNCTION sync_inventory_uuid_shadow();

DROP TRIGGER IF EXISTS trg_sync_orders_uuid_shadow ON orders;
CREATE TRIGGER trg_sync_orders_uuid_shadow
    BEFORE INSERT OR UPDATE OF id, new_id, listing_id, new_listing_id, buyer_id, new_buyer_id, seller_id, new_seller_id
    ON orders
    FOR EACH ROW
    EXECUTE FUNCTION sync_orders_uuid_shadow();

CREATE OR REPLACE VIEW uuid_shadow_divergence AS
SELECT
    'users'::TEXT AS relation_name,
    COUNT(*) FILTER (WHERE new_id IS NULL)::BIGINT AS missing_shadow_ids,
    0::BIGINT AS fk_drift_rows
FROM users
UNION ALL
SELECT
    'inventory'::TEXT AS relation_name,
    COUNT(*) FILTER (WHERE i.new_id IS NULL OR i.new_owner_id IS NULL)::BIGINT AS missing_shadow_ids,
    COUNT(*) FILTER (WHERE u.id IS NULL OR i.new_owner_id IS DISTINCT FROM u.new_id)::BIGINT AS fk_drift_rows
FROM inventory AS i
LEFT JOIN users AS u ON u.id = i.owner_id
UNION ALL
SELECT
    'orders'::TEXT AS relation_name,
    COUNT(*) FILTER (
        WHERE o.new_id IS NULL
           OR o.new_listing_id IS NULL
           OR o.new_buyer_id IS NULL
           OR o.new_seller_id IS NULL
    )::BIGINT AS missing_shadow_ids,
    COUNT(*) FILTER (
        WHERE i.id IS NULL
           OR buyer.id IS NULL
           OR seller.id IS NULL
           OR o.new_listing_id IS DISTINCT FROM i.new_id
           OR o.new_buyer_id IS DISTINCT FROM buyer.new_id
           OR o.new_seller_id IS DISTINCT FROM seller.new_id
    )::BIGINT AS fk_drift_rows
FROM orders AS o
LEFT JOIN inventory AS i ON i.id = o.listing_id
LEFT JOIN users AS buyer ON buyer.id = o.buyer_id
LEFT JOIN users AS seller ON seller.id = o.seller_id;
