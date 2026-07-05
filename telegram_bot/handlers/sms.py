from aiogram import Router
from aiogram.filters import Command
from aiogram.types import Message

from services.api import api

router = Router()


@router.message(Command("send_sms"))
async def cmd_send_sms(message: Message):
    parts = message.text.split(maxsplit=2)
    if len(parts) < 3:
        await message.answer("❌ Использование: /send_sms &lt;номер&gt; &lt;текст&gt;")
        return

    phone = parts[1].strip()
    text = parts[2].strip()

    try:
        result = await api.request(
            "POST",
            "/bot/v1/sms/send",
            json={
                "phone": phone,
                "message": text,
                "telegram_id": message.from_user.id,
            },
        )
    except Exception as e:
        await message.answer(f"❌ Ошибка отправки SMS: {e}")
        return

    status = "✅" if result.get("success") else "❌"
    provider_response = result.get("provider_response", {})
    await message.answer(
        f"{status} <b>Статус:</b> {result.get('message')}\n"
        f"<b>Ответ шлюза:</b> <code>{provider_response}</code>",
        parse_mode="HTML",
    )
