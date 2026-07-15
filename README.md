# ElohimSMS

Сервис отправки SMS с Telegram-интерфейсом и переключением между провайдерами.

## Состав проекта

- `rust_backend` — HTTP API, PostgreSQL, отправка SMS и failover провайдеров;
- `telegram_bot` — административный Telegram-бот;
- `captcha_site_example` — пример страницы для коротких ссылок;
- `docker-compose.yml` — локальный запуск всего стека.

## Быстрый запуск

1. Скопируйте `.env.example` в `.env` и заполните обязательные значения.
2. Запустите сервисы:

   ```shell
   docker compose up --build
   ```

Backend будет доступен на `http://localhost:3000`, Swagger UI — на
`http://localhost:3000/swagger-ui/`.

PostgreSQL хранится в Docker volume `postgres_data`. Локальные данные базы,
логи и секреты не должны добавляться в Git.

## Проверки

```shell
cd rust_backend
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test

cd ../telegram_bot
ruff check .
ruff format --check .
python -m compileall -q .
```

Эти же проверки выполняются в CI при push и pull request.
