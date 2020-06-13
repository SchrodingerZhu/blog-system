-- This file should undo anything in `up.sql`

ALTER TABLE posts DROP COLUMN text_searchable;

DROP TRIGGER tsvectorupdateproducts ON posts;