from aiogram import Router
from aiogram.filters import Command
from aiogram.types import Message

from services.api import api

router = Router()


@router.message(Command("templates"))
async def cmd_templates(message: Message):
    try:
        templates = await api.list_templates()
    except Exception as e:
        await message.answer(f"❌ Ошибка получения шаблонов: {e}")
        return

    if not templates:
        await message.answer(
            "📭 Шаблонов пока нет.\n\nДобавьте через:\n/set_template US Hello! Click: {link}"
        )
        return

    lines = ["📝 <b>Список шаблонов SMS:</b>"]
    for t in templates:
        status = "🟢" if t.get("is_active") else "🔴"
        lines.append(f"{status} <b>{t['country_code']}</b>\n<code>{t['text']}</code>")

    await message.answer("\n\n".join(lines), parse_mode="HTML")


@router.message(Command("set_template"))
async def cmd_set_template(message: Message):
    parts = message.text.split(maxsplit=2)
    if len(parts) < 3:
        await message.answer(
            "❌ Использование: /set_template &lt;код_страны&gt; &lt;текст&gt;\n\n"
            "Пример:\n"
            "/set_template US Hello! Click here: {link}\n\n"
            "Доступные placeholders:\n"
            "{link} — короткая ссылка\n"
            "{phone} — номер получателя\n"
            "{country} — код страны",
            parse_mode="HTML",
        )
        return

    country_code = parts[1].strip().upper()
    text = parts[2].strip()

    if len(country_code) != 2:
        await message.answer(
            "❌ Код страны должен состоять из 2 букв (например, US, GB, DE)."
        )
        return

    try:
        template = await api.create_or_update_template(country_code, text)
    except Exception as e:
        await message.answer(f"❌ Ошибка сохранения шаблона: {e}")
        return

    await message.answer(
        f"✅ Шаблон для <b>{template['country_code']}</b> сохранён.\n\n"
        f"<code>{template['text']}</code>",
        parse_mode="HTML",
    )


@router.message(Command("delete_template"))
async def cmd_delete_template(message: Message):
    parts = message.text.split(maxsplit=1)
    if len(parts) < 2:
        await message.answer("❌ Использование: /delete_template &lt;код_страны&gt;")
        return

    country_code = parts[1].strip().upper()

    try:
        await api.delete_template(country_code)
    except Exception as e:
        await message.answer(f"❌ Ошибка удаления шаблона: {e}")
        return

    await message.answer(
        f"✅ Шаблон для <b>{country_code}</b> удалён.", parse_mode="HTML"
    )
