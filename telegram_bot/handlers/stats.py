from aiogram import Router, types

from handlers.common import BTN_STATS, main_menu_keyboard
from services.api import api

router = Router()


@router.message(lambda m: m.text == BTN_STATS)
async def btn_stats(message: types.Message):
    try:
        stats = await api.get_stats()
    except Exception as e:
        await message.answer(f"❌ Ошибка получения статистики: {e}")
        return

    await message.answer(
        f"📊 <b>Статистика сервиса:</b>\n\n"
        f"👥 Администраторов: <b>{stats.get('admins_count', 'N/A')}</b>\n"
        f"🔑 API-ключей всего: <b>{stats.get('keys_total', 'N/A')}</b>\n"
        f"🟢 Активных ключей: <b>{stats.get('keys_active', 'N/A')}</b>\n"
        f"💰 Баланс шлюза: <b>{stats.get('balance', 'N/A')}</b>",
        reply_markup=main_menu_keyboard(),
        parse_mode="HTML",
    )
