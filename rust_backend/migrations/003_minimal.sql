-- Упрощаем схему после удаления неиспользуемых endpoint'ов:
-- admin/keys/templates/campaigns/links и поля telegram_id.
DROP TABLE IF EXISTS campaigns CASCADE;
DROP TABLE IF EXISTS templates CASCADE;
DROP TABLE IF EXISTS api_keys CASCADE;
DROP TABLE IF EXISTS admins CASCADE;

ALTER TABLE sms_logs
    DROP COLUMN IF EXISTS telegram_id,
    DROP COLUMN IF EXISTS campaign_id,
    DROP COLUMN IF EXISTS api_key_id;
