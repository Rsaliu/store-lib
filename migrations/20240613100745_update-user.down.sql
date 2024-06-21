-- Add down migration script here
ALTER TABLE users
DROP COLUMN email,
DROP COLUMN confirmed,
DROP COLUMN created_at,
DROP COLUMN updated_at;