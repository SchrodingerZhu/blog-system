-- Your SQL goes here
CREATE EXTENSION citext;

ALTER TABLE posts
ALTER COLUMN title SET DATA TYPE citext;

ALTER TABLE pages
ALTER COLUMN title SET DATA TYPE citext;

