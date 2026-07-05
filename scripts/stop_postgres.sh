#!/usr/bin/env bash
set -e

PG_VERSION=${PG_VERSION:-16}
PG_BIN="/c/Program Files/PostgreSQL/${PG_VERSION}/bin"
DATA_DIR="${DATA_DIR:-postgres_data}"

export PATH="$PG_BIN:$PATH"

echo "Stopping PostgreSQL..."
pg_ctl -D "$DATA_DIR" stop || true
echo "PostgreSQL stopped."
