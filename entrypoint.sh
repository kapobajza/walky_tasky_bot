#!/bin/sh

if [ -f "/run/secrets/walky-tasky-bot-db-password" ]; then
    export DB_PASSWORD=$(cat /run/secrets/walky-tasky-bot-db-password)
fi

if [ -f "/run/secrets/walky-tasky-bot-telegram-token" ]; then
    export TELOXIDE_TOKEN=$(cat /run/secrets/walky-tasky-bot-telegram-token)
fi

exec "$@"