-- Webhook, привязанный к конкретному API-ключу.
--
-- webhook_secret хранится открыто, потому что backend использует его для
-- формирования HMAC-подписи. В production его желательно шифровать
-- отдельным master key приложения.
ALTER TABLE api_keys
    ADD COLUMN IF NOT EXISTS webhook_url TEXT,
    ADD COLUMN IF NOT EXISTS webhook_secret TEXT,
    ADD COLUMN IF NOT EXISTS webhook_is_active BOOLEAN NOT NULL DEFAULT FALSE;

-- external_id позволяет внешнему боту связать событие ElohimSMS
-- со своим заказом, пользователем, диалогом или иной сущностью.
ALTER TABLE campaigns
    ADD COLUMN IF NOT EXISTS external_id TEXT,
    ADD COLUMN IF NOT EXISTS first_clicked_at TIMESTAMPTZ;

-- Не отправляем "первое" уведомление для старых кампаний,
-- по которым переход уже был зафиксирован до установки миграции.
UPDATE campaigns
SET first_clicked_at = COALESCE(sent_at, created_at)
WHERE click_count > 0
  AND first_clicked_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_campaigns_external_id
    ON campaigns(api_key_id, external_id)
    WHERE external_id IS NOT NULL;
