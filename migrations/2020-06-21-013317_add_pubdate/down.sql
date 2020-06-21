-- This file should undo anything in `up.sql`
ALTER table posts
DROP COLUMN public_date;

ALTER TABLE posts
RENAME update_date TO date;
