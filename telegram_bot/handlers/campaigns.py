from aiogram import Router
from aiogram.filters import Command
from aiogram.types import Message

from services.api import api

router = Router()


@router.message(Command("send_campaign"))
async def cmd_send_campaign(message: Message):
    parts = message.text.split(maxsplit=2)
    if len(parts) < 3:
        await message.answer(
            "❌ Использование: /send_campaign &lt;номер&gt; &lt;ссылка&gt;\n\n"
            "Пример:\n"
            "/send_campaign +19005551234 https://example.com"
        )
        return

    phone = parts[1].strip()
    url = parts[2].strip()

    await message.answer("⏳ Отправка кампании...")

    try:
        result = await api.send_campaign(phone, url, message.from_user.id)
    except Exception as e:
        await message.answer(f"❌ Ошибка отправки кампании: {e}")
        return

    status = "✅" if result.get("success") else "❌"
    await message.answer(
        f"{status} <b>Кампания отправлена</b>\n\n"
        f"<b>ID:</b> <code>{result.get('campaign_id')}</code>\n"
        f"<b>Короткая ссылка:</b> {result.get('short_link')}\n"
        f"<b>Сообщение:</b> <code>{result.get('message')}</code>\n\n"
        f"<b>Ответ шлюза:</b> <code>{result.get('provider_response')}</code>",
        parse_mode="HTML",
    )
