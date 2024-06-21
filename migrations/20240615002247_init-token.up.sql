-- Add up migration script here
CREATE TABLE
    "tokens" (
        id       uuid primary key default gen_random_uuid(),
        token_string      text unique not null,
        user_id uuid      not null,
        expired_in TIMESTAMP not null,
        created_at TIMESTAMP not null default now(),
        updated_at TIMESTAMP not null default now()
    );