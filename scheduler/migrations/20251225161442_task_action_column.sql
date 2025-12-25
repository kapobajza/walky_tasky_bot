-- Add migration script here

ALTER TABLE tasks
ADD COLUMN IF NOT EXISTS action JSONB NOT NULL DEFAULT '{}'::JSONB;

