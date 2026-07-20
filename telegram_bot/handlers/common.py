from aiogram import Router, types
from aiogram.filters import Command
from aiogram.types import ReplyKeyboardMarkup, KeyboardButton
from aiogram.fsm.context import FSMContext

from services.api import api

router = Router()


async def user_is_owner(user_id: int) -> bool:
    try:
        admins = await api.list_admins()
    except Exception:
        return False
    return any(
        a.get("telegram_id") == user_id and a.get("is_owner") for a in admins
    )

BTN_STATS = "📊 Статистика"
BTN_ADMINS = "👥 Администраторы"
BTN_KEYS = "🔑 API-ключи"
BTN_TEMPLATES = "📝 Шаблоны"
BTN_SENDER_NAMES = "📛 Имена отправителя"
BTN_SEND_SMS = "📤 Отправить SMS"
BTN_SETTINGS = "⚙️ Настройки"
BTN_HELP = "❓ Помощь"
BTN_MENU = "🏠 Меню"
BTN_BACK = "◀️ Назад"


def main_menu_keyboard(is_owner: bool = False) -> ReplyKeyboardMarkup:
    buttons = [
        [KeyboardButton(text=BTN_STATS)],
        [KeyboardButton(text=BTN_KEYS), KeyboardButton(text=BTN_TEMPLATES)],
        [KeyboardButton(text=BTN_SENDER_NAMES)],
        [KeyboardButton(text=BTN_SEND_SMS)],
        [KeyboardButton(text=BTN_SETTINGS), KeyboardButton(text=BTN_HELP)],
    ]
    if is_owner:
        buttons.insert(1, [KeyboardButton(text=BTN_ADMINS)])
    return ReplyKeyboardMarkup(
        keyboard=buttons,
        resize_keyboard=True,
        input_field_placeholder="Выберите действие",
    )


def cancel_menu_keyboard() -> ReplyKeyboardMarkup:
    """Клавиатура с кнопками Назад и Меню для выхода из любого диалога."""
    return ReplyKeyboardMarkup(
        keyboard=[
            [KeyboardButton(text=BTN_BACK), KeyboardButton(text=BTN_MENU)],
        ],
        resize_keyboard=True,
    )


async def go_to_main_menu(message: types.Message, state: FSMContext) -> None:
    await state.clear()
    is_owner = await user_is_owner(message.from_user.id)
    await message.answer(
        "Главное меню",
        reply_markup=main_menu_keyboard(is_owner=is_owner),
    )


async def handle_control_buttons(message: types.Message, state: FSMContext) -> bool:
    """Обрабатывает кнопки Назад/Меню и не-текстовые сообщения.

    Возвращает True, если дальнейшая обработка не нужна.
    """
    if not message.text:
        await message.answer(
            "❌ Пожалуйста, отправьте текстовое сообщение.",
            reply_markup=cancel_menu_keyboard(),
        )
        return True
    text = message.text.strip()
    if text in (BTN_BACK, BTN_MENU):
        await go_to_main_menu(message, state)
        return True
    return False


@router.message(Command("start"))
async def cmd_start(message: types.Message, state: FSMContext):
    await state.clear()
    is_owner = await user_is_owner(message.from_user.id)
    await message.answer(
        "👋 Добро пожаловать в <b>ElohimSMS Bot</b>.\n\n"
        "Используйте меню ниже для управления сервисом рассылки SMS.",
        reply_markup=main_menu_keyboard(is_owner=is_owner),
        parse_mode="HTML",
    )


@router.message(lambda m: m.text and m.text.strip() == BTN_MENU)
@router.message(Command("menu"))
async def btn_menu(message: types.Message, state: FSMContext):
    await go_to_main_menu(message, state)


@router.message(lambda m: m.text and m.text.strip() == BTN_BACK)
async def btn_back(message: types.Message, state: FSMContext):
    # По умолчанию Назад = отмена и выход в главное меню.
    # Конкретные FSM-обработчики могут перехватывать BTN_BACK раньше.
    await go_to_main_menu(message, state)


@router.message(Command("help"))
@router.message(lambda m: m.text and m.text.strip() == BTN_HELP)
async def cmd_help(message: types.Message):
    await message.answer(
        "📋 <b>Как пользоваться ботом</b>:\n\n"
        "• <b>Администраторы</b> — управление доступом (только владелец).\n"
        "• <b>API-ключи</b> — создание и отзыв ключей для API.\n"
        "• <b>Шаблоны</b> — храните именованные шаблоны на страну и выбирайте избранный.\n"
        "• <b>Отправить SMS</b> — ручная отправка сообщения или по шаблону.\n"
        "  Используйте <code>{link}</code> в тексте — он заменится на короткую ссылку.\n"
        "• <b>Настройки</b> — имя отправителя (sender ID).\n\n"
        "Placeholders: <code>{link}</code>, <code>{phone}</code>, <code>{country}</code>\n\n"
        "Кнопки <b>◀️ Назад</b> и <b>🏠 Меню</b> работают в любом диалоге.",
        parse_mode="HTML",
    )
