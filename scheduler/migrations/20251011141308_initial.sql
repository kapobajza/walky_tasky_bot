-- Add migration script here

CREATE TABLE IF NOT EXISTS tasks (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    schedule_type SMALLINT NOT NULL,
    schedule TEXT,
    last_run TIMESTAMPTZ,
    next_run TIMESTAMPTZ NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    retry_count INT NOT NULL DEFAULT 0,
    retry_delay INT NOT NULL DEFAULT 1000, -- in milliseconds
    max_retries INT NOT NULL DEFAULT 3,

    CONSTRAINT pk_tasks PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS users (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    username TEXT,
    chat_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,

    CONSTRAINT pk_users PRIMARY KEY (id),
    CONSTRAINT uq_username UNIQUE (username),
    CONSTRAINT uq_chat_id UNIQUE (chat_id),
    CONSTRAINT uq_user_id UNIQUE (user_id)
);

CREATE TABLE IF NOT EXISTS user_tasks (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    task_id UUID NOT NULL,

    CONSTRAINT pk_user_tasks PRIMARY KEY (id),
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_task FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);