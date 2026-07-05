from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env", env_file_encoding="utf-8", extra="ignore"
    )

    bot_token: str
    api_base_url: str = "http://localhost:3000"
    internal_bot_token: str
    owner_telegram_id: int

    # Режим работы: polling или webhook
    bot_mode: str = "polling"

    # Настройки webhook (используются только при bot_mode=webhook)
    webhook_url: str = ""
    webhook_host: str = "0.0.0.0"
    webhook_port: int = 8080
    webhook_path: str = "/webhook"


settings = Settings()
