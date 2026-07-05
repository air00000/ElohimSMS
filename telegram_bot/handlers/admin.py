from aiogram import Router
from aiogram.filters import Command
from aiogram.types import Message

from services.api import api

router = Router()


async def is_owner(user_id: int) -> bool:
    admins = await api.list_admins()
    for admin in admins:
        if admin["telegram_id"] == user_id:
            return admin.get("is_owner", False)
    return False


@router.message(Command("list_admins"))
async def cmd_list_admins(message: Message):
    try:
        admins = await api.list_admins()
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    if not admins:
        await message.answer("Администраторов пока нет.")
        return

    lines = ["👥 <b>Список администраторов:</b>"]
    for admin in admins:
        owner_badge = " 👑" if admin.get("is_owner") else ""
        username = f" @{admin.get('username')}" if admin.get("username") else ""
        lines.append(f"• <code>{admin['telegram_id']}</code>{username}{owner_badge}")

    await message.answer("\n".join(lines), parse_mode="HTML")


@router.message(Command("add_admin"))
async def cmd_add_admin(message: Message):
    if not await is_owner(message.from_user.id):
        await message.answer("⛔ Только владелец может назначать администраторов.")
        return

    parts = message.text.split(maxsplit=2)
    if len(parts) < 2:
        await message.answer(
            "❌ Использование: /add_admin &lt;telegram_id&gt; [username]"
        )
        return

    try:
        telegram_id = int(parts[1])
    except ValueError:
        await message.answer("❌ telegram_id должен быть числом.")
        return

    username = parts[2] if len(parts) > 2 else None

    try:
        admin = await api.create_admin(telegram_id, username)
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    await message.answer(
        f"✅ Администратор <code>{admin['telegram_id']}</code> добавлен.",
        parse_mode="HTML",
    )


@router.message(Command("remove_admin"))
async def cmd_remove_admin(message: Message):
    if not await is_owner(message.from_user.id):
        await message.answer("⛔ Только владелец может снимать администраторов.")
        return

    parts = message.text.split(maxsplit=1)
    if len(parts) < 2:
        await message.answer("❌ Использование: /remove_admin &lt;telegram_id&gt;")
        return

    try:
        telegram_id = int(parts[1])
    except ValueError:
        await message.answer("❌ telegram_id должен быть числом.")
        return

    if telegram_id == message.from_user.id:
        await message.answer("❌ Вы не можете удалить самого себя.")
        return

    try:
        await api.remove_admin(telegram_id)
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    await message.answer(
        f"✅ Администратор <code>{telegram_id}</code> удалён.",
        parse_mode="HTML",
    )
