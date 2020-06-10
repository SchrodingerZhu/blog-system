-- Your SQL goes here

ALTER TABLE posts
    ADD COLUMN text_searchable tsvector NOT NULL;

UPDATE posts
SET text_searchable =
        to_tsvector('english', title || ' ' || content);

CREATE INDEX textsearch_idx ON posts USING GIN (text_searchable);

CREATE TRIGGER tsvectorupdateproducts
    BEFORE INSERT OR UPDATE
    ON posts
    FOR EACH ROW
EXECUTE PROCEDURE
    tsvector_update_trigger(text_searchable, 'pg_catalog.english', title, content);


