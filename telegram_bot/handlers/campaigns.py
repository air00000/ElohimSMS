from aiogram import Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup

from handlers.common import (
    BTN_BACK,
    BTN_MENU,
    BTN_SEND_CAMPAIGN,
    cancel_menu_keyboard,
    go_to_main_menu,
    main_menu_keyboard,
)
from services.api import api

router = Router()


class CampaignFSM(StatesGroup):
    waiting_phone = State()
    waiting_url = State()


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
    if message.text == BTN_MENU:
        await go_to_main_menu(message, state)
        return
    if message.text == BTN_BACK:
        await go_to_main_menu(message, state)
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
    if message.text == BTN_MENU:
        await go_to_main_menu(message, state)
        return
    if message.text == BTN_BACK:
        await state.set_state(CampaignFSM.waiting_phone)
        await message.answer(
            "Введите номер телефона получателя в международном формате:",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    url = message.text.strip()
    if not url.startswith(("http://", "https://")):
        await message.answer(
            "❌ Ссылка должна начинаться с http:// или https://",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    data = await state.get_data()
    phone = data["phone"]

    await message.answer("⏳ Отправка кампании...")

    try:
        result = await api.send_campaign(phone, url, message.from_user.id)
    except Exception as e:
        await message.answer(f"❌ Ошибка отправки кампании: {e}")
        return
    finally:
        await state.clear()

    status = "✅" if result.get("success") else "❌"
    await message.answer(
        f"{status} <b>Кампания отправлена</b>\n\n"
        f"<b>ID:</b> <code>{result.get('campaign_id')}</code>\n"
        f"<b>Короткая ссылка:</b> {result.get('short_link')}\n"
        f"<b>Сообщение:</b> <code>{result.get('message')}</code>\n\n"
        f"<b>Ответ шлюза:</b> <code>{result.get('provider_response')}</code>",
        reply_markup=main_menu_keyboard(),
        parse_mode="HTML",
    )
