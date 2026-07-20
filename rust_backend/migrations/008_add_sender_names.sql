-- Имена отправителя (sender ID) по странам, избранное на страну — зеркало templates.

CREATE TABLE IF NOT EXISTS sender_names (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    country_code VARCHAR(2) NOT NULL,
    name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_favorite BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sender_names_country ON sender_names(country_code);
CREATE INDEX IF NOT EXISTS idx_sender_names_favorite ON sender_names(country_code, is_favorite);
