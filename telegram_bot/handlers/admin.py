from aiogram import F, Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup
from aiogram.types import InlineKeyboardMarkup, InlineKeyboardButton
from aiogram.utils.keyboard import InlineKeyboardBuilder

from handlers.common import (
    BTN_ADMINS,
    cancel_menu_keyboard,
    go_to_main_menu,
    handle_control_buttons,
    main_menu_keyboard,
    user_is_owner,
)
from services.api import api

router = Router()


class AddAdminFSM(StatesGroup):
    waiting_id = State()


@router.message(lambda m: m.text == BTN_ADMINS)
async def btn_admins(message: types.Message):
    if not await user_is_owner(message.from_user.id):
        await message.answer("⛔ Только владелец может управлять администраторами.")
        return

    try:
        admins = await api.list_admins()
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    if not admins:
        await message.answer(
            "Администраторов пока нет.",
            reply_markup=InlineKeyboardMarkup(
                inline_keyboard=[
                    [InlineKeyboardButton(text="➕ Добавить админа", callback_data="admin:add")]
                ]
            ),
        )
        return

    lines = ["👥 <b>Список администраторов:</b>"]
    for admin in admins:
        owner_badge = " 👑" if admin.get("is_owner") else ""
        username = f" @{admin.get('username')}" if admin.get("username") else ""
        lines.append(
            f"• <code>{admin['telegram_id']}</code>{username}{owner_badge}"
        )

    builder = InlineKeyboardBuilder()
    builder.button(text="➕ Добавить админа", callback_data="admin:add")
    for admin in admins:
        if not admin.get("is_owner"):
            builder.button(
                text=f"❌ Удалить {admin['telegram_id']}",
                callback_data=f"admin:remove:{admin['telegram_id']}",
            )
    builder.adjust(1, 1)

    await message.answer(
        "\n".join(lines),
        reply_markup=builder.as_markup(),
        parse_mode="HTML",
    )


@router.callback_query(F.data == "admin:add")
async def cb_admin_add(query: types.CallbackQuery, state: FSMContext):
    if not await user_is_owner(query.from_user.id):
        await query.answer("⛔ Только владелец может добавлять администраторов.", show_alert=True)
        return

    await state.set_state(AddAdminFSM.waiting_id)
    await query.message.answer(
        "Введите <b>telegram_id</b> нового администратора.\n"
        "Можно сразу через пробел указать username (без @).\n\n"
        "Пример: <code>123456789 ivanov</code>",
        reply_markup=cancel_menu_keyboard(),
        parse_mode="HTML",
    )
    await query.answer()


@router.message(AddAdminFSM.waiting_id)
async def process_admin_id(message: types.Message, state: FSMContext):
    if not await user_is_owner(message.from_user.id):
        await state.clear()
        await message.answer("⛔ Только владелец может добавлять администраторов.")
        return

    if await handle_control_buttons(message, state):
        return

    parts = message.text.strip().split(maxsplit=1)
    if not parts:
        await message.answer(
            "❌ Введите telegram_id.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    try:
        telegram_id = int(parts[0])
    except ValueError:
        await message.answer(
            "❌ telegram_id должен быть числом.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    username = parts[1].strip() if len(parts) > 1 else None

    try:
        admin = await api.create_admin(telegram_id, username)
    except Exception as e:
        await state.clear()
        await message.answer(
            f"❌ Ошибка: {e}",
            reply_markup=main_menu_keyboard(is_owner=True),
        )
        return

    await state.clear()
    await message.answer(
        f"✅ Администратор <code>{admin['telegram_id']}</code> добавлен.",
        reply_markup=main_menu_keyboard(is_owner=True),
        parse_mode="HTML",
    )


@router.callback_query(F.data.startswith("admin:remove:"))
async def cb_admin_remove(query: types.CallbackQuery):
    if not await user_is_owner(query.from_user.id):
        await query.answer("⛔ Только владелец может удалять администраторов.", show_alert=True)
        return

    telegram_id = int(query.data.split(":")[-1])

    try:
        await api.remove_admin(telegram_id)
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    await query.answer("Администратор удалён")
    await query.message.answer(
        f"✅ Администратор <code>{telegram_id}</code> удалён.",
        parse_mode="HTML",
    )
