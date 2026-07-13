-- Добавляем имя провайдера, через которого фактически отправилось SMS,
-- чтобы видеть распределение между основным и резервными сервисами.

ALTER TABLE sms_logs
    ADD COLUMN IF NOT EXISTS provider_name TEXT;

ALTER TABLE campaigns
    ADD COLUMN IF NOT EXISTS provider_name TEXT;
