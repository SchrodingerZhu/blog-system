-- This file should undo anything in `up.sql`

ALTER TABLE posts DROP COLUMN text_searchable;

DROP TRIGGER tsvector_update ON posts;