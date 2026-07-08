from collections import defaultdict

from aiogram import F, Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup
from aiogram.types import InlineKeyboardMarkup, InlineKeyboardButton
from aiogram.utils.keyboard import InlineKeyboardBuilder

from handlers.common import BTN_TEMPLATES, main_menu_keyboard
from services.api import api

router = Router()


class AddTemplateFSM(StatesGroup):
    waiting_country = State()
    waiting_text = State()


@router.message(lambda m: m.text == BTN_TEMPLATES)
async def btn_templates(message: types.Message):
    try:
        templates = await api.list_templates()
    except Exception as e:
        await message.answer(f"❌ Ошибка получения шаблонов: {e}")
        return

    if not templates:
        await message.answer(
            "📭 Шаблонов пока нет.",
            reply_markup=InlineKeyboardMarkup(
                inline_keyboard=[
                    [InlineKeyboardButton(text="➕ Добавить шаблон", callback_data="templ:add")]
                ]
            ),
        )
        return

    # Группируем по странам
    by_country: dict[str, list[dict]] = defaultdict(list)
    for t in templates:
        by_country[t["country_code"]].append(t)

    lines = ["📝 <b>Шаблоны по странам:</b>"]
    for country in sorted(by_country):
        favorite = next(
            (t for t in by_country[country] if t.get("is_favorite")), None
        )
        mark = " ⭐" if favorite else ""
        lines.append(f"• <b>{country}</b>{mark} — {len(by_country[country])} шт.")

    builder = InlineKeyboardBuilder()
    builder.button(text="➕ Добавить шаблон", callback_data="templ:add")
    for country in sorted(by_country):
        builder.button(text=country, callback_data=f"templ:country:{country}")
    builder.adjust(1, 2)

    await message.answer(
        "\n".join(lines),
        reply_markup=builder.as_markup(),
        parse_mode="HTML",
    )


@router.callback_query(F.data.startswith("templ:country:"))
async def cb_templates_country(query: types.CallbackQuery):
    country = query.data.split(":")[-1]
    try:
        templates = await api.list_templates()
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    country_templates = [t for t in templates if t["country_code"] == country]
    if not country_templates:
        await query.answer("Шаблоны не найдены")
        return

    lines = [f"📝 <b>Шаблоны для {country}:</b>"]
    for t in country_templates:
        star = " ⭐" if t.get("is_favorite") else ""
        name = f" <i>{t.get('name')}</i>" if t.get("name") else ""
        lines.append(f"{star}{name}\n<code>{t['text']}</code>")

    builder = InlineKeyboardBuilder()
    for t in country_templates:
        if not t.get("is_favorite"):
            builder.button(
                text=f"⭐ {t['id'][:8]}",
                callback_data=f"templ:fav:{t['id']}",
            )
        builder.button(
            text=f"🗑 {t['id'][:8]}",
            callback_data=f"templ:del:{t['id']}",
        )
    builder.button(text="➕ Добавить", callback_data="templ:add")
    builder.adjust(2, 1)

    await query.message.answer(
        "\n\n".join(lines),
        reply_markup=builder.as_markup(),
        parse_mode="HTML",
    )
    await query.answer()


@router.callback_query(F.data == "templ:add")
async def cb_template_add(query: types.CallbackQuery, state: FSMContext):
    await state.set_state(AddTemplateFSM.waiting_country)
    await query.message.answer(
        "Введите двухбуквенный код страны, например <code>US</code>, <code>GB</code>, <code>DE</code>:",
        parse_mode="HTML",
    )
    await query.answer()


@router.message(AddTemplateFSM.waiting_country)
async def process_template_country(message: types.Message, state: FSMContext):
    country = message.text.strip().upper()
    if len(country) != 2 or not country.isalpha():
        await message.answer("❌ Код страны должен состоять из 2 букв.")
        return

    await state.update_data(country=country)
    await state.set_state(AddTemplateFSM.waiting_text)
    await message.answer(
        "Введите текст шаблона.\n\n"
        "Доступные placeholders:\n"
        "<code>{link}</code> — короткая ссылка\n"
        "<code>{phone}</code> — номер получателя\n"
        "<code>{country}</code> — код страны",
        parse_mode="HTML",
    )


@router.message(AddTemplateFSM.waiting_text)
async def process_template_text(message: types.Message, state: FSMContext):
    text = message.text.strip()
    if not text:
        await message.answer("❌ Текст шаблона не может быть пустым.")
        return

    data = await state.get_data()
    country = data["country"]

    try:
        template = await api.create_template(country, text)
    except Exception as e:
        await message.answer(f"❌ Ошибка сохранения шаблона: {e}")
        return
    finally:
        await state.clear()

    await message.answer(
        f"✅ Шаблон для <b>{template['country_code']}</b> сохранён.\n\n"
        f"<code>{template['text']}</code>\n\n"
        "Если это первый шаблон для страны, он автоматически стал избранным.",
        reply_markup=main_menu_keyboard(),
        parse_mode="HTML",
    )


@router.callback_query(F.data.startswith("templ:fav:"))
async def cb_template_favorite(query: types.CallbackQuery):
    template_id = query.data.split(":")[-1]
    try:
        template = await api.set_favorite_template(template_id)
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    await query.answer("Сделано избранным")
    await query.message.answer(
        f"⭐ Шаблон для <b>{template['country_code']}</b> теперь избранный.",
        parse_mode="HTML",
    )


@router.callback_query(F.data.startswith("templ:del:"))
async def cb_template_delete(query: types.CallbackQuery):
    template_id = query.data.split(":")[-1]
    try:
        await api.delete_template(template_id)
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    await query.answer("Шаблон удалён")
    await query.message.answer("✅ Шаблон удалён.")
