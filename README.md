# ElohimSMS

Сервис фишинг-рассылки SMS через Telegram-бота и собственное REST API. Интегрирован с SMS-шлюзом [Devil-Traff](https://api.devil-traff.cc) и Google reCAPTCHA v2.

## Архитектура

Проект состоит из двух микросервисов:

- **`rust_backend/`** — REST API на Rust (axum + sqlx + PostgreSQL). Отвечает за:
  - приём кампаний (`phone` + `url`) от клиентов по API,
  - определение страны по номеру телефона,
  - подбор SMS-шаблона по стране,
  - генерацию коротких ссылок,
  - проверку Google reCAPTCHA v2 и выдачу целевого URL,
  - отправку SMS через Devil-Traff,
  - управление администраторами, API-ключами и шаблонами,
  - логирование отправленных SMS.
- **`telegram_bot/`** — Telegram-бот на Python (aiogram 3). Используется для:
  - тестовой отправки кампаний,
  - управления SMS-шаблонами по странам.
- **`captcha_site_example/`** — пример внешнего сайта с Google reCAPTCHA v2.

## Как это работает

1. Клиент отправляет запрос:
   ```json
   POST /api/v1/campaigns/send
   { "phone": "+19005551234", "url": "https://example.com" }
   ```
2. Backend определяет страну по номеру (US).
3. Backend находит SMS-шаблон для страны US.
4. Backend генерирует короткую ссылку: `https://your-captcha-site.com/l/AbCdEfGh`.
5. Backend подставляет ссылку в шаблон и отправляет SMS через Devil-Traff.
6. Получатель переходит по короткой ссылке на внешний сайт с Google reCAPTCHA v2.
7. После прохождения капчи сайт запрашивает у backend целевой URL и редиректит пользователя.

## Требования

- [PostgreSQL](https://www.postgresql.org/download/) 14+
- [Rust](https://www.rust-lang.org/tools/install) 1.79+
- [Python](https://www.python.org/downloads/) 3.11+

## Быстрый старт (локально, без Docker)

### 1. Установить PostgreSQL

Windows (через winget):

```powershell
winget install --id PostgreSQL.PostgreSQL.16 --accept-package-agreements --accept-source-agreements
```

Или скачайте установщик с [официального сайта](https://www.postgresql.org/download/windows/).

### 2. Запустить локальный кластер PostgreSQL

```bash
bash scripts/start_postgres.sh
```

Скрипт создаст папку `postgres_data/`, запустит PostgreSQL на порту `5433` и создаст базу `elohim_sms`.

Остановить позже:

```bash
bash scripts/stop_postgres.sh
```

### 3. Настроить переменные окружения

```bash
cp .env.example .env
```

Заполните обязательные переменные:

| Переменная | Описание |
|------------|----------|
| `BOT_TOKEN` | Токен Telegram-бота от [@BotFather](https://t.me/botfather) |
| `OWNER_TELEGRAM_ID` | Telegram ID владельца бота |
| `INTERNAL_BOT_TOKEN` | Случайная строка для внутренней авторизации |
| `SMS_GATEWAY_AUTH_TOKEN` | JWT-токен от [Devil-Traff](https://api.devil-traff.cc) |
| `SMS_GATEWAY_ROUTE` | Маршрут отправки, например `Auto` |
| `SMS_GATEWAY_SENDER_ID` | Имя отправителя |
| `CAPTCHA_SITE_URL` | Домен внешнего сайта с капчей |
| `RECAPTCHA_SECRET` | Секретный ключ Google reCAPTCHA v2 |
| `RECAPTCHA_SITE_KEY` | Публичный ключ Google reCAPTCHA v2 |

### 4. Запустить backend

```bash
cd rust_backend
cargo run --release
```

Backend будет доступен на http://localhost:3000.

### 5. Запустить Telegram-бота

В новом терминале:

```bash
cd telegram_bot
python -m venv .venv
source .venv/bin/activate  # Windows Git Bash: source .venv/Scripts/activate
pip install -r requirements.txt
cp ../.env .env
python main.py
```

### 6. Настроить внешний сайт с капчей

Скопируйте `captcha_site_example/index.html` на свой хостинг, замените `BACKEND_URL` и `RECAPTCHA_SITE_KEY`. Убедитесь, что домен добавлен в настройках Google reCAPTCHA.

### 7. Проверить работу

```bash
curl http://localhost:3000/health
```

Swagger UI: http://localhost:3000/swagger-ui

## API

### Публичные endpoints

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/health` | Проверка работоспособности |
| GET | `/swagger-ui` | Swagger UI |
| GET | `/api/v1/links/:short_code` | Информация о короткой ссылке |
| POST | `/api/v1/links/:short_code/verify` | Проверка Google reCAPTCHA v2 + целевой URL |

### Endpoints для клиентов (требуется `X-API-Key`)

| Метод | Путь | Описание |
|-------|------|----------|
| POST | `/api/v1/campaigns/send` | Отправить кампанию |
| POST | `/api/v1/sms/send` | Отправить обычное SMS |
| GET | `/api/v1/sms/balance` | Баланс Devil-Traff |
| GET | `/api/v1/sms/routes` | Доступные маршруты |

Пример отправки кампании:

```bash
curl -X POST http://localhost:3000/api/v1/campaigns/send \
  -H "Content-Type: application/json" \
  -H "X-API-Key: ваш_ключ" \
  -d '{"phone":"+19005551234","url":"https://example.com"}'
```

### Endpoints для Telegram-бота (требуется `X-Internal-Bot-Token`)

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/bot/v1/admin` | Список админов |
| POST | `/bot/v1/admin` | Добавить админа |
| DELETE | `/bot/v1/admin/:id` | Удалить админа |
| GET | `/bot/v1/keys` | Список ключей |
| POST | `/bot/v1/keys` | Создать ключ |
| POST | `/bot/v1/keys/:id/revoke` | Отозвать ключ |
| GET | `/bot/v1/templates` | Список шаблонов |
| POST | `/bot/v1/templates` | Создать/обновить шаблон |
| DELETE | `/bot/v1/templates/:country_code` | Удалить шаблон |
| POST | `/bot/v1/campaigns/send` | Тестовая отправка кампании |

## Команды Telegram-бота

| Команда | Описание |
|---------|----------|
| `/start` | Приветствие |
| `/help` | Список команд |
| `/templates` | Список SMS-шаблонов |
| `/set_template <US> <текст>` | Сохранить шаблон |
| `/delete_template <US>` | Удалить шаблон |
| `/send_campaign <номер> <ссылка>` | Отправить тестовую кампанию |
| `/send_sms <номер> <текст>` | Отправить обычное SMS |
| `/list_keys` / `/create_key` / `/revoke_key` | Управление API-ключами |
| `/list_admins` / `/add_admin` / `/remove_admin` | Управление админами |
| `/stats` | Статистика |

## Placeholders в шаблонах

- `{link}` — короткая ссылка на страницу с капчей
- `{phone}` — номер получателя
- `{country}` — код страны

Пример:
```
/set_template US Hello! Confirm your account: {link}
```

## Важные замечания

- Devil-Traff не поддерживает все страны. Перед отправкой проверяйте маршруты через `/api/v1/sms/routes`.
- Для локального тестирования reCAPTCHA нужно добавить `localhost` в список доменов в панели Google reCAPTCHA.
- Внешний сайт с капчей — отдельный сервис, backend предоставляет только API.

## CI/CD

GitHub Actions запускает:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo build --release`
- `ruff check .`
- `ruff format --check .`
