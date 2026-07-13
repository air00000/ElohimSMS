from aiogram import Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup

from handlers.common import (
    BTN_BACK,
    BTN_MENU,
    BTN_SEND_CAMPAIGN,
    cancel_menu_keyboard,
    go_to_main_menu,
    handle_control_buttons,
    main_menu_keyboard,
    user_is_owner,
)
from services.api import api

router = Router()

BTN_DEFAULT_SENDER = "🔤 По умолчанию (TRACKING)"


class CampaignFSM(StatesGroup):
    waiting_phone = State()
    waiting_url = State()
    waiting_sender_id = State()


def _sender_id_keyboard() -> types.ReplyKeyboardMarkup:
    return types.ReplyKeyboardMarkup(
        keyboard=[
            [types.KeyboardButton(text=BTN_DEFAULT_SENDER)],
            [types.KeyboardButton(text=BTN_BACK), types.KeyboardButton(text=BTN_MENU)],
        ],
        resize_keyboard=True,
    )


@router.message(lambda m: m.text == BTN_SEND_CAMPAIGN)
async def btn_send_campaign(message: types.Message, state: FSMContext):
    await state.set_state(CampaignFSM.waiting_phone)
    await message.answer(
        "📨 <b>Отправка кампании</b>\n\n"
        "Введите номер телефона получателя в международном формате:",
        reply_markup=cancel_menu_keyboard(),
        parse_mode="HTML",
    )


@router.message(CampaignFSM.waiting_phone)
async def process_campaign_phone(message: types.Message, state: FSMContext):
    if await handle_control_buttons(message, state):
        return

    phone = message.text.strip()
    if not phone.startswith("+"):
        await message.answer(
            "❌ Номер должен начинаться с '+' и кода страны.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    await state.update_data(phone=phone)
    await state.set_state(CampaignFSM.waiting_url)
    await message.answer(
        "Введите целевую ссылку (URL):",
        reply_markup=cancel_menu_keyboard(),
    )


@router.message(CampaignFSM.waiting_url)
async def process_campaign_url(message: types.Message, state: FSMContext):
    if not message.text:
        await message.answer(
            "❌ Пожалуйста, отправьте текстовое сообщение.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    text = message.text.strip()
    if text == BTN_BACK:
        await state.set_state(CampaignFSM.waiting_phone)
        await message.answer(
            "Введите номер телефона получателя в международном формате:",
            reply_markup=cancel_menu_keyboard(),
        )
        return
    if text == BTN_MENU:
        await go_to_main_menu(message, state)
        return

    url = text
    if not url.startswith(("http://", "https://")):
        await message.answer(
            "❌ Ссылка должна начинаться с http:// или https://",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    await state.update_data(url=url)
    await state.set_state(CampaignFSM.waiting_sender_id)
    await message.answer(
        "Введите имя отправителя (Sender ID).\n\n"
        "Максимум 11 латинских букв/цифр. Нажмите <b>По умолчанию (TRACKING)</b>, "
        "чтобы оставить стандартное значение.",
        reply_markup=_sender_id_keyboard(),
        parse_mode="HTML",
    )


@router.message(CampaignFSM.waiting_sender_id)
async def process_campaign_sender_id(message: types.Message, state: FSMContext):
    if not message.text:
        await message.answer(
            "❌ Пожалуйста, отправьте текстовое сообщение.",
            reply_markup=_sender_id_keyboard(),
        )
        return

    text = message.text.strip()
    if text == BTN_BACK:
        await state.set_state(CampaignFSM.waiting_url)
        await message.answer(
            "Введите целевую ссылку (URL):",
            reply_markup=cancel_menu_keyboard(),
        )
        return
    if text == BTN_MENU:
        await go_to_main_menu(message, state)
        return

    sender_id = "TRACKING" if text == BTN_DEFAULT_SENDER else text
    if not sender_id:
        await message.answer(
            "❌ Имя отправителя не может быть пустым.",
            reply_markup=_sender_id_keyboard(),
        )
        return
    if len(sender_id) > 11:
        await message.answer(
            "❌ Имя отправителя не должно превышать 11 символов.",
            reply_markup=_sender_id_keyboard(),
        )
        return

    data = await state.get_data()
    phone = data.get("phone")
    url = data.get("url")
    if not phone or not url:
        await state.clear()
        is_owner = await user_is_owner(message.from_user.id)
        await message.answer(
            "❌ Данные диалога утеряны. Начните заново.",
            reply_markup=main_menu_keyboard(is_owner=is_owner),
        )
        return

    await message.answer("⏳ Отправка кампании...")

    try:
        result = await api.send_campaign(
            phone, url, message.from_user.id, sender_id=sender_id
        )
    except Exception as e:
        await state.clear()
        is_owner = await user_is_owner(message.from_user.id)
        await message.answer(
            f"❌ Ошибка отправки кампании: {e}",
            reply_markup=main_menu_keyboard(is_owner=is_owner),
        )
        return

    await state.clear()

    status = "✅" if result.get("success") else "❌"
    is_owner = await user_is_owner(message.from_user.id)
    await message.answer(
        f"{status} <b>Кампания отправлена</b>\n\n"
        f"<b>ID:</b> <code>{result.get('campaign_id')}</code>\n"
        f"<b>Короткая ссылка:</b> {result.get('short_link')}\n"
        f"<b>Сообщение:</b> <code>{result.get('message')}</code>\n\n"
        f"<b>Ответ шлюза:</b> <code>{result.get('provider_response')}</code>",
        reply_markup=main_menu_keyboard(is_owner=is_owner),
        parse_mode="HTML",
    )
