-- Your SQL goes here

CREATE TRIGGER tsvector_update
    BEFORE INSERT OR UPDATE
    ON posts
    FOR EACH ROW
EXECUTE PROCEDURE
    tsvector_update_trigger(text_searchable, 'pg_catalog.english', title, content);

DROP TRIGGER tsvectorupdateproducts ON posts;