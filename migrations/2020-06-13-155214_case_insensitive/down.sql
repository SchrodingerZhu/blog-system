-- This file should undo anything in `up.sql`

ALTER TABLE posts
    ALTER COLUMN title SET DATA TYPE text;

ALTER TABLE pages
    ALTER COLUMN title SET DATA TYPE text;

DROP EXTENSION citext;