-- Your SQL goes here
ALTER TABLE posts
ADD COLUMN public_date timestamp NOT NULL default current_timestamp;

UPDATE posts
    SET public_date = date;

ALTER TABLE posts
RENAME date TO update_date;
