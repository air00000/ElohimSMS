from aiogram import Router
from aiogram.filters import Command
from aiogram.types import Message

from services.api import api

router = Router()


@router.message(Command("stats"))
async def cmd_stats(message: Message):
    try:
        stats = await api.get_stats()
    except Exception as e:
        await message.answer(f"❌ Ошибка получения статистики: {e}")
        return

    await message.answer(
        f"📊 <b>Статистика сервиса:</b>\n\n"
        f"👥 Администраторов: <b>{stats['admins_count']}</b>\n"
        f"🔑 API-ключей всего: <b>{stats['keys_total']}</b>\n"
        f"💰 Баланс шлюза: <b>{stats['balance']}</b>",
        parse_mode="HTML",
    )
