from aiogram import F, Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup
from aiogram.types import InlineKeyboardMarkup, InlineKeyboardButton
from aiogram.utils.keyboard import InlineKeyboardBuilder

from handlers.common import (
    BTN_BACK,
    BTN_KEYS,
    BTN_MENU,
    cancel_menu_keyboard,
    go_to_main_menu,
    main_menu_keyboard,
)
from services.api import api

router = Router()


class CreateKeyFSM(StatesGroup):
    waiting_name = State()


@router.message(lambda m: m.text == BTN_KEYS)
async def btn_keys(message: types.Message):
    try:
        keys = await api.list_keys()
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    if not keys:
        await message.answer(
            "API-ключей пока нет.",
            reply_markup=InlineKeyboardMarkup(
                inline_keyboard=[
                    [InlineKeyboardButton(text="➕ Создать ключ", callback_data="key:create")]
                ]
            ),
        )
        return

    lines = [f"🔑 <b>Список API-ключей</b> (всего: {len(keys)}):"]
    for key in keys:
        status = "🟢" if key.get("is_active") else "🔴"
        created = key.get("created_at", "")[:10]
        lines.append(
            f"{status} <b>{key['name']}</b>\n"
            f"   ID: <code>{key['id']}</code>\n"
            f"   Создан: {created}"
        )

    builder = InlineKeyboardBuilder()
    builder.button(text="➕ Создать ключ", callback_data="key:create")
    for key in keys:
        if key.get("is_active"):
            builder.button(
                text=f"❌ Отозвать {key['name']}",
                callback_data=f"key:revoke:{key['id']}",
            )
    builder.adjust(1, 1)

    await message.answer(
        "\n\n".join(lines),
        reply_markup=builder.as_markup(),
        parse_mode="HTML",
    )


@router.callback_query(F.data == "key:create")
async def cb_key_create(query: types.CallbackQuery, state: FSMContext):
    await state.set_state(CreateKeyFSM.waiting_name)
    await query.message.answer(
        "Введите название для нового API-ключа:",
        reply_markup=cancel_menu_keyboard(),
    )
    await query.answer()


@router.message(CreateKeyFSM.waiting_name)
async def process_key_name(message: types.Message, state: FSMContext):
    if message.text in (BTN_BACK, BTN_MENU):
        await go_to_main_menu(message, state)
        return

    name = message.text.strip()
    if not name:
        await message.answer(
            "❌ Название не может быть пустым.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    try:
        key = await api.create_key(name, message.from_user.id)
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return
    finally:
        await state.clear()

    await message.answer(
        "✅ API-ключ создан.\n\n"
        f"<b>Название:</b> {key['name']}\n"
        f"<b>ID:</b> <code>{key['id']}</code>\n"
        f"<b>Ключ:</b> <code>{key['key']}</code>\n\n"
        "⚠️ Сохраните ключ сейчас, он больше не будет показан.",
        reply_markup=main_menu_keyboard(),
        parse_mode="HTML",
    )


@router.callback_query(F.data.startswith("key:revoke:"))
async def cb_key_revoke(query: types.CallbackQuery):
    key_id = query.data.split(":")[-1]
    try:
        await api.revoke_key(key_id)
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    await query.answer("Ключ отозван")
    await query.message.answer(
        f"✅ Ключ <code>{key_id}</code> отозван.",
        parse_mode="HTML",
    )
