-- Your SQL goes here
ALTER TABLE posts
ADD CONSTRAINT post_title_name_constraint CHECK ( title ~* '^[ \da-zA-Z]+$' );

ALTER TABLE pages
ADD CONSTRAINT page_title_name_constraint CHECK ( title ~* '^[ \da-zA-Z]+$' );
