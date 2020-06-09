-- Your SQL goes here
CREATE TABLE comments (
    id SERIAL PRIMARY KEY NOT NULL ,
    post_id SERIAL NOT NULL,
    nickname VARCHAR NOT NULL ,
    email VARCHAR NOT NULL CONSTRAINT proper_email CHECK (email ~* '^[A-Za-z0-9._%-]+@[A-Za-z0-9.-]+[.][A-Za-z]+$'),
    content TEXT NOT NULL,
    signature TEXT NOT NULL,
    finger_print VARCHAR NOT NULL
)