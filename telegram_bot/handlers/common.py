from aiogram import Router, types
from aiogram.filters import Command
from aiogram.types import ReplyKeyboardMarkup, KeyboardButton

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
BTN_SEND_SMS = "📤 Отправить SMS"
BTN_SEND_CAMPAIGN = "📨 Отправить кампанию"
BTN_SETTINGS = "⚙️ Настройки"
BTN_HELP = "❓ Помощь"
BTN_MENU = "🏠 Главное меню"


def main_menu_keyboard(is_owner: bool = False) -> ReplyKeyboardMarkup:
    buttons = [
        [KeyboardButton(text=BTN_STATS)],
        [KeyboardButton(text=BTN_KEYS), KeyboardButton(text=BTN_TEMPLATES)],
        [KeyboardButton(text=BTN_SEND_SMS), KeyboardButton(text=BTN_SEND_CAMPAIGN)],
        [KeyboardButton(text=BTN_SETTINGS), KeyboardButton(text=BTN_HELP)],
    ]
    if is_owner:
        buttons.insert(1, [KeyboardButton(text=BTN_ADMINS)])
    return ReplyKeyboardMarkup(
        keyboard=buttons,
        resize_keyboard=True,
        input_field_placeholder="Выберите действие",
    )


@router.message(Command("start"))
async def cmd_start(message: types.Message):
    is_owner = await user_is_owner(message.from_user.id)
    await message.answer(
        "👋 Добро пожаловать в <b>ElohimSMS Bot</b>.\n\n"
        "Используйте меню ниже для управления сервисом рассылки SMS.",
        reply_markup=main_menu_keyboard(is_owner=is_owner),
        parse_mode="HTML",
    )


@router.message(lambda m: m.text == BTN_MENU)
async def btn_menu(message: types.Message):
    await message.answer(
        "Главное меню",
        reply_markup=main_menu_keyboard(),
    )


@router.message(Command("help"))
@router.message(lambda m: m.text == BTN_HELP)
async def cmd_help(message: types.Message):
    await message.answer(
        "📋 <b>Как пользоваться ботом</b>:\n\n"
        "• <b>Администраторы</b> — управление доступом (только владелец).\n"
        "• <b>API-ключи</b> — создание и отзыв ключей для API.\n"
        "• <b>Шаблоны</b> — храните несколько шаблонов на страну и выбирайте избранный.\n"
        "• <b>Отправить SMS</b> — ручная отправка сообщения.\n"
        "• <b>Отправить кампанию</b> — отправка по избранному шаблону с короткой ссылкой.\n"
        "• <b>Настройки</b> — имя отправителя (sender ID).\n\n"
        "Placeholders в шаблонах: <code>{link}</code>, <code>{phone}</code>, <code>{country}</code>",
        parse_mode="HTML",
    )
