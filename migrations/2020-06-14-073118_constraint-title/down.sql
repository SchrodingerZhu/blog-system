-- This file should undo anything in `up.sql`
ALTER TABLE posts
DROP CONSTRAINT post_title_name_constraint;

ALTER TABLE pages
DROP CONSTRAINT page_title_name_constraint;