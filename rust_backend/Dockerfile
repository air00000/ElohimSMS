FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

# Установка зависимостей для сборки.
# curl нужен для utoipa-swagger-ui, чтобы скачать Swagger UI во время компиляции.
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/elohim_sms_backend /app/elohim_sms_backend
COPY --from=builder /app/migrations /app/migrations

EXPOSE 3000

CMD ["./elohim_sms_backend"]
