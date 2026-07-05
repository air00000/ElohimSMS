#!/usr/bin/env bash
set -e

# Скрипт для запуска локального кластера PostgreSQL.
# Требуется установленный PostgreSQL (например, через winget или с официального сайта).
# Для Windows запускайте через Git Bash.

PG_VERSION=${PG_VERSION:-16}
PG_BIN="/c/Program Files/PostgreSQL/${PG_VERSION}/bin"
DATA_DIR="${DATA_DIR:-postgres_data}"
PORT="${PORT:-5433}"
LOG_FILE="${LOG_FILE:-postgres.log}"

if [ ! -d "$PG_BIN" ]; then
    echo "ERROR: PostgreSQL binaries not found at $PG_BIN"
    echo "Please install PostgreSQL ${PG_VERSION} or set PG_VERSION to installed version."
    exit 1
fi

export PATH="$PG_BIN:$PATH"

if [ ! -d "$DATA_DIR" ]; then
    echo "Initializing PostgreSQL cluster in $DATA_DIR..."
    initdb -D "$DATA_DIR" -U postgres -A trust --locale=en_US.UTF-8
fi

if pg_isready -h localhost -p "$PORT" >/dev/null 2>&1; then
    echo "PostgreSQL is already running on port $PORT"
else
    echo "Starting PostgreSQL on port $PORT..."
    pg_ctl -D "$DATA_DIR" -l "$LOG_FILE" -W start -o "-p $PORT"
    sleep 2
fi

if pg_isready -h localhost -p "$PORT" >/dev/null 2>&1; then
    echo "PostgreSQL is ready on localhost:$PORT"
    echo "Creating database 'elohim_sms' if not exists..."
    psql -h localhost -p "$PORT" -U postgres -tc "SELECT 1 FROM pg_database WHERE datname = 'elohim_sms'" | grep -q 1 || \
        psql -h localhost -p "$PORT" -U postgres -c "CREATE DATABASE elohim_sms;"
    echo "Done. Use DATABASE_URL=postgres://postgres@localhost:$PORT/elohim_sms"
else
    echo "ERROR: PostgreSQL failed to start. Check $LOG_FILE"
    exit 1
fi
