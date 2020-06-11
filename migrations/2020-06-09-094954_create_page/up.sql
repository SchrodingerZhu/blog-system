-- Your SQL goes here
CREATE TABLE pages (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL UNIQUE,
    content TEXT NOT NULL,
    important BOOLEAN NOT NULL
)