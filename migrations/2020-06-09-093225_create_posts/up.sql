-- Your SQL goes here
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL UNIQUE,
    date TIMESTAMP NOT NULL,
    tags text[] NOT NULL DEFAULT '{}',
    content text NOT NULL,
    signature text NOT NULL
)