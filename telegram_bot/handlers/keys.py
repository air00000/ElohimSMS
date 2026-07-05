from aiogram import Router
from aiogram.filters import Command
from aiogram.types import Message

from services.api import api

router = Router()


@router.message(Command("list_keys"))
async def cmd_list_keys(message: Message):
    try:
        result = await api.list_keys()
        keys = result.get("data", [])
        total = result.get("total", 0)
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    if not keys:
        await message.answer("API-ключей пока нет.")
        return

    lines = [f"🔑 <b>Список API-ключей</b> (всего: {total}):"]
    for key in keys:
        status = "🟢" if key.get("is_active") else "🔴"
        created = key.get("created_at", "")[:10]
        lines.append(
            f"{status} <b>{key['name']}</b>\n"
            f"   ID: <code>{key['id']}</code>\n"
            f"   Создан: {created}"
        )

    await message.answer("\n\n".join(lines), parse_mode="HTML")


@router.message(Command("create_key"))
async def cmd_create_key(message: Message):
    parts = message.text.split(maxsplit=1)
    if len(parts) < 2:
        await message.answer("❌ Использование: /create_key &lt;название&gt;")
        return

    name = parts[1].strip()

    try:
        key = await api.create_key(name, message.from_user.id)
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    await message.answer(
        f"✅ API-ключ создан.\n\n"
        f"<b>Название:</b> {key['name']}\n"
        f"<b>ID:</b> <code>{key['id']}</code>\n"
        f"<b>Ключ:</b> <code>{key['key']}</code>\n\n"
        f"⚠️ Сохраните ключ сейчас, он больше не будет показан.",
        parse_mode="HTML",
    )


@router.message(Command("revoke_key"))
async def cmd_revoke_key(message: Message):
    parts = message.text.split(maxsplit=1)
    if len(parts) < 2:
        await message.answer("❌ Использование: /revoke_key &lt;id&gt;")
        return

    key_id = parts[1].strip()

    try:
        await api.revoke_key(key_id)
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    await message.answer(f"✅ Ключ <code>{key_id}</code> отозван.", parse_mode="HTML")
