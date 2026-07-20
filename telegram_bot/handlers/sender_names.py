from collections import defaultdict

from aiogram import F, Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup
from aiogram.types import InlineKeyboardMarkup, InlineKeyboardButton
from aiogram.utils.keyboard import InlineKeyboardBuilder

from handlers.common import (
    BTN_SENDER_NAMES,
    cancel_menu_keyboard,
    handle_control_buttons,
    main_menu_keyboard,
    user_is_owner,
)
from services.api import api

router = Router()


class AddSenderNameFSM(StatesGroup):
    waiting_country = State()
    waiting_name = State()


@router.message(lambda m: m.text == BTN_SENDER_NAMES)
async def btn_sender_names(message: types.Message):
    try:
        sender_names = await api.list_sender_names()
    except Exception as e:
        await message.answer(f"❌ Ошибка получения имён отправителя: {e}")
        return

    if not sender_names:
        await message.answer(
            "📭 Имён отправителя пока нет.",
            reply_markup=InlineKeyboardMarkup(
                inline_keyboard=[
                    [InlineKeyboardButton(text="➕ Добавить имя отправителя", callback_data="snames:add")]
                ]
            ),
        )
        return

    # Группируем по странам
    by_country: dict[str, list[dict]] = defaultdict(list)
    for s in sender_names:
        by_country[s["country_code"]].append(s)

    lines = ["📛 <b>Имена отправителя по странам:</b>"]
    for country in sorted(by_country):
        favorite = next(
            (s for s in by_country[country] if s.get("is_favorite")), None
        )
        mark = " ⭐" if favorite else ""
        lines.append(f"• <b>{country}</b>{mark} — {len(by_country[country])} шт.")

    builder = InlineKeyboardBuilder()
    builder.button(text="➕ Добавить имя отправителя", callback_data="snames:add")
    for country in sorted(by_country):
        builder.button(text=country, callback_data=f"snames:country:{country}")
    builder.adjust(1, 2)

    await message.answer(
        "\n".join(lines),
        reply_markup=builder.as_markup(),
        parse_mode="HTML",
    )


@router.callback_query(F.data.startswith("snames:country:"))
async def cb_sender_names_country(query: types.CallbackQuery):
    country = query.data.split(":")[-1]
    try:
        sender_names = await api.list_sender_names()
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    country_names = [s for s in sender_names if s["country_code"] == country]
    if not country_names:
        await query.answer("Имена отправителя не найдены")
        return

    lines = [f"📛 <b>Имена отправителя для {country}:</b>"]
    for s in country_names:
        star = " ⭐" if s.get("is_favorite") else ""
        lines.append(f"{star}<b>{s['name']}</b>")

    builder = InlineKeyboardBuilder()
    for s in country_names:
        if not s.get("is_favorite"):
            builder.button(
                text=f"⭐ {s['name']}",
                callback_data=f"snames:fav:{s['id']}",
            )
        builder.button(
            text=f"🗑 {s['name']}",
            callback_data=f"snames:del:{s['id']}",
        )
    builder.button(text="➕ Добавить", callback_data="snames:add")
    builder.adjust(2, 1)

    await query.message.answer(
        "\n\n".join(lines),
        reply_markup=builder.as_markup(),
        parse_mode="HTML",
    )
    await query.answer()


@router.callback_query(F.data == "snames:add")
async def cb_sender_name_add(query: types.CallbackQuery, state: FSMContext):
    await state.set_state(AddSenderNameFSM.waiting_country)
    await query.message.answer(
        "Введите двухбуквенный код страны, например <code>US</code>, <code>GB</code>, <code>DE</code>:",
        reply_markup=cancel_menu_keyboard(),
        parse_mode="HTML",
    )
    await query.answer()


@router.message(AddSenderNameFSM.waiting_country)
async def process_sender_name_country(message: types.Message, state: FSMContext):
    if await handle_control_buttons(message, state):
        return

    country = message.text.strip().upper()
    if len(country) != 2 or not country.isalpha():
        await message.answer(
            "❌ Код страны должен состоять из 2 букв.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    await state.update_data(country=country)
    await state.set_state(AddSenderNameFSM.waiting_name)
    await message.answer(
        "Введите имя отправителя (sender ID) — до 11 символов, латинские буквы и цифры:",
        reply_markup=cancel_menu_keyboard(),
    )


@router.message(AddSenderNameFSM.waiting_name)
async def process_sender_name_name(message: types.Message, state: FSMContext):
    if await handle_control_buttons(message, state):
        return

    name = message.text.strip()
    if not name or len(name) > 11 or not name.isascii() or not name.isalnum():
        await message.answer(
            "❌ Имя отправителя должно быть от 1 до 11 символов: латинские буквы и цифры.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    data = await state.get_data()
    country = data.get("country")
    if not country:
        await state.clear()
        is_owner = await user_is_owner(message.from_user.id)
        await message.answer(
            "❌ Данные диалога утеряны. Начните заново.",
            reply_markup=main_menu_keyboard(is_owner=is_owner),
        )
        return

    try:
        sender_name = await api.create_sender_name(country, name)
    except Exception as e:
        await state.clear()
        is_owner = await user_is_owner(message.from_user.id)
        await message.answer(
            f"❌ Ошибка сохранения имени отправителя: {e}",
            reply_markup=main_menu_keyboard(is_owner=is_owner),
        )
        return

    await state.clear()
    is_owner = await user_is_owner(message.from_user.id)
    await message.answer(
        f"✅ Имя отправителя <b>{sender_name['name']}</b> для <b>{sender_name['country_code']}</b> сохранено.\n\n"
        "Если это первое имя для страны, оно автоматически стало избранным.",
        reply_markup=main_menu_keyboard(is_owner=is_owner),
        parse_mode="HTML",
    )


@router.callback_query(F.data.startswith("snames:fav:"))
async def cb_sender_name_favorite(query: types.CallbackQuery):
    sender_name_id = query.data.split(":")[-1]
    try:
        sender_name = await api.set_favorite_sender_name(sender_name_id)
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    await query.answer("Сделано избранным")
    await query.message.answer(
        f"⭐ Имя отправителя <b>{sender_name['name']}</b> для <b>{sender_name['country_code']}</b> теперь избранное.",
        parse_mode="HTML",
    )


@router.callback_query(F.data.startswith("snames:del:"))
async def cb_sender_name_delete(query: types.CallbackQuery):
    sender_name_id = query.data.split(":")[-1]
    try:
        await api.delete_sender_name(sender_name_id)
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    await query.answer("Имя отправителя удалено")
    await query.message.answer("✅ Имя отправителя удалено.")
