from aiogram import F, Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup

from handlers.common import BTN_SEND_SMS, main_menu_keyboard
from services.api import api

router = Router()


class SendSmsFSM(StatesGroup):
    waiting_phone = State()
    waiting_text = State()


@router.message(lambda m: m.text == BTN_SEND_SMS)
async def btn_send_sms(message: types.Message, state: FSMContext):
    await state.set_state(SendSmsFSM.waiting_phone)
    await message.answer(
        "📤 <b>Отправка SMS</b>\n\nВведите номер телефона в международном формате:",
        parse_mode="HTML",
    )


@router.message(SendSmsFSM.waiting_phone)
async def process_sms_phone(message: types.Message, state: FSMContext):
    phone = message.text.strip()
    if not phone.startswith("+"):
        await message.answer("❌ Номер должен начинаться с '+' и кода страны.")
        return

    await state.update_data(phone=phone)
    await state.set_state(SendSmsFSM.waiting_text)
    await message.answer("Введите текст сообщения:")


@router.message(SendSmsFSM.waiting_text)
async def process_sms_text(message: types.Message, state: FSMContext):
    text = message.text.strip()
    if not text:
        await message.answer("❌ Текст не может быть пустым.")
        return

    data = await state.get_data()
    phone = data["phone"]

    await message.answer("⏳ Отправка SMS...")

    try:
        result = await api.send_sms(phone, text, message.from_user.id)
    except Exception as e:
        await message.answer(f"❌ Ошибка отправки SMS: {e}")
        return
    finally:
        await state.clear()

    status = "✅" if result.get("success") else "❌"
    await message.answer(
        f"{status} <b>Статус:</b> {result.get('message')}\n"
        f"<b>Ответ шлюза:</b> <code>{result.get('provider_response')}</code>",
        reply_markup=main_menu_keyboard(),
        parse_mode="HTML",
    )
