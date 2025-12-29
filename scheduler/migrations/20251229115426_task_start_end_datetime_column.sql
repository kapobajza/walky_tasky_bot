-- Add migration script here

ALTER TABLE tasks
ADD COLUMN start_date TIMESTAMPTZ,
ADD COLUMN end_date TIMESTAMPTZ,
DROP COLUMN IF EXISTS schedule;
