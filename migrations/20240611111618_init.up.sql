-- Add up migration script here
CREATE TYPE user_role AS ENUM ('normal', 'admin');
CREATE TABLE
    "users" (
        id       uuid primary key default gen_random_uuid(),
        username      text unique not null,
        password_hash text        not null,
        user_role      user_role not null
    );