-- Add up migration script here
ALTER TABLE users
ADD COLUMN email TEXT NOT NULL DEFAULT 'noemail@yahoo.com',
ADD COLUMN confirmed BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN created_at TIMESTAMP not null default now(),
ADD COLUMN updated_at TIMESTAMP not null default now();