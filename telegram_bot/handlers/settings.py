from aiogram import Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup

from handlers.common import BTN_SETTINGS, main_menu_keyboard
from services.api import api

router = Router()


class SettingsFSM(StatesGroup):
    waiting_sender_name = State()


@router.message(lambda m: m.text == BTN_SETTINGS)
async def btn_settings(message: types.Message):
    try:
        admins = await api.list_admins()
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return

    admin = next(
        (a for a in admins if a.get("telegram_id") == message.from_user.id), None
    )
    sender_name = admin.get("sender_name") if admin else None
    sender_name = sender_name or "(не задано, используется значение шлюза)"

    await message.answer(
        f"⚙️ <b>Настройки</b>\n\n"
        f"<b>Имя отправителя (sender ID):</b> <code>{sender_name}</code>\n\n"
        "Это имя будет использоваться при отправке SMS из бота и через API-ключи, "
        "созданные вами.\n\n"
        "Нажмите кнопку ниже, чтобы изменить.",
        reply_markup=types.InlineKeyboardMarkup(
            inline_keyboard=[
                [types.InlineKeyboardButton(text="✏️ Изменить sender name", callback_data="settings:sender")]
            ]
        ),
        parse_mode="HTML",
    )


@router.callback_query(lambda c: c.data == "settings:sender")
async def cb_settings_sender(query: types.CallbackQuery, state: FSMContext):
    await state.set_state(SettingsFSM.waiting_sender_name)
    await query.message.answer(
        "Введите новое имя отправителя (sender ID).\n"
        "Или отправьте <code>-</code>, чтобы сбросить значение по умолчанию.",
        parse_mode="HTML",
    )
    await query.answer()


@router.message(SettingsFSM.waiting_sender_name)
async def process_sender_name(message: types.Message, state: FSMContext):
    raw = message.text.strip()
    sender_name = None if raw == "-" else raw

    try:
        await api.update_sender_name(message.from_user.id, sender_name)
    except Exception as e:
        await message.answer(f"❌ Ошибка: {e}")
        return
    finally:
        await state.clear()

    await message.answer(
        f"✅ Имя отправителя обновлено: <code>{sender_name or 'по умолчанию'}</code>",
        reply_markup=main_menu_keyboard(),
        parse_mode="HTML",
    )
