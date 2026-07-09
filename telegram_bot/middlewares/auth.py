from collections.abc import Awaitable, Callable
from typing import Any

from aiogram import BaseMiddleware
from aiogram.types import CallbackQuery, Message, TelegramObject

from services.api import api


class AdminMiddleware(BaseMiddleware):
    async def __call__(
        self,
        handler: Callable[[TelegramObject, dict[str, Any]], Awaitable[Any]],
        event: TelegramObject,
        data: dict[str, Any],
    ) -> Any:
        user = None
        if isinstance(event, Message):
            user = event.from_user
        elif isinstance(event, CallbackQuery):
            user = event.from_user

        if user is None:
            return await handler(event, data)

        try:
            admins = await api.list_admins()
        except Exception as e:
            if isinstance(event, CallbackQuery):
                await event.answer(f"❌ Ошибка проверки прав: {e}", show_alert=True)
            elif isinstance(event, Message):
                await event.answer(f"❌ Ошибка проверки прав: {e}")
            return None

        admin_ids = {admin["telegram_id"] for admin in admins}
        if user.id not in admin_ids:
            if isinstance(event, CallbackQuery):
                await event.answer("⛔ У вас нет прав администратора.", show_alert=True)
            elif isinstance(event, Message):
                await event.answer("⛔ У вас нет прав администратора.")
            return None

        return await handler(event, data)
