-- This file should undo anything in `up.sql`

DROP TRIGGER tsvector_update ON posts;

CREATE TRIGGER tsvectorupdateproducts
    BEFORE INSERT OR UPDATE
    ON posts
    FOR EACH ROW
EXECUTE PROCEDURE
    tsvector_update_trigger(text_searchable, 'pg_catalog.english', title, content);
