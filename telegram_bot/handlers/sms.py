from aiogram import F, Router, types
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup
from aiogram.utils.keyboard import InlineKeyboardBuilder

from handlers.common import (
    BTN_BACK,
    BTN_MENU,
    BTN_SEND_SMS,
    cancel_menu_keyboard,
    go_to_main_menu,
    main_menu_keyboard,
    user_is_owner,
)
from services.api import api

router = Router()

BTN_MANUAL = "✍️ Набрать вручную"
BTN_TEMPLATE = "📋 Выбрать шаблон"
BTN_DEFAULT_SENDER = "🔤 По умолчанию (TRACKING)"


class SendSmsFSM(StatesGroup):
    waiting_phone = State()
    waiting_choose = State()
    waiting_text = State()
    waiting_url = State()
    waiting_sender_id = State()


# Префиксы → коды стран. Должны совпадать с логикой backend.
PHONE_PREFIXES: list[tuple[str, str]] = [
    ("994", "AZ"),
    ("375", "BY"),
    ("374", "AM"),
    ("373", "MD"),
    ("372", "EE"),
    ("371", "LV"),
    ("370", "LT"),
    ("380", "UA"),
    ("992", "TJ"),
    ("995", "GE"),
    ("996", "KG"),
    ("998", "UZ"),
    ("972", "IL"),
    ("971", "AE"),
    ("966", "SA"),
    ("49", "DE"),
    ("44", "GB"),
    ("33", "FR"),
    ("39", "IT"),
    ("34", "ES"),
    ("41", "CH"),
    ("43", "AT"),
    ("31", "NL"),
    ("32", "BE"),
    ("45", "DK"),
    ("46", "SE"),
    ("47", "NO"),
    ("48", "PL"),
    ("90", "TR"),
    ("20", "EG"),
    ("27", "ZA"),
    ("91", "IN"),
    ("92", "PK"),
    ("86", "CN"),
    ("81", "JP"),
    ("82", "KR"),
    ("65", "SG"),
    ("61", "AU"),
    ("64", "NZ"),
    ("7", "RU"),
    ("1", "US"),
]


def detect_country_code(phone: str) -> str | None:
    """Определяет двухбуквенный код страны по номеру телефона."""
    digits = "".join(c for c in phone if c.isdigit())
    for prefix, country in PHONE_PREFIXES:
        if digits.startswith(prefix):
            return country
    return None


def _choose_keyboard(country_code: str | None) -> types.ReplyKeyboardMarkup:
    keyboard_buttons = [[types.KeyboardButton(text=BTN_MANUAL)]]
    if country_code:
        keyboard_buttons.append([types.KeyboardButton(text=BTN_TEMPLATE)])
    keyboard_buttons.append([
        types.KeyboardButton(text=BTN_BACK),
        types.KeyboardButton(text=BTN_MENU),
    ])
    return types.ReplyKeyboardMarkup(
        keyboard=keyboard_buttons,
        resize_keyboard=True,
    )


def _sender_id_keyboard() -> types.ReplyKeyboardMarkup:
    return types.ReplyKeyboardMarkup(
        keyboard=[
            [types.KeyboardButton(text=BTN_DEFAULT_SENDER)],
            [types.KeyboardButton(text=BTN_BACK), types.KeyboardButton(text=BTN_MENU)],
        ],
        resize_keyboard=True,
    )


async def _require_text(message: types.Message) -> str | None:
    if not message.text:
        await message.answer(
            "❌ Пожалуйста, отправьте текстовое сообщение.",
            reply_markup=cancel_menu_keyboard(),
        )
        return None
    return message.text.strip()


async def _get_state_data_or_cancel(
    message: types.Message, state: FSMContext, *keys: str
) -> dict | None:
    data = await state.get_data()
    missing = [k for k in keys if not data.get(k)]
    if missing:
        await state.clear()
        is_owner = await user_is_owner(message.from_user.id)
        await message.answer(
            "❌ Данные диалога утеряны. Начните заново.",
            reply_markup=main_menu_keyboard(is_owner=is_owner),
        )
        return None
    return data


@router.message(lambda m: m.text == BTN_SEND_SMS)
async def btn_send_sms(message: types.Message, state: FSMContext):
    await state.set_state(SendSmsFSM.waiting_phone)
    await message.answer(
        "📤 <b>Отправка SMS</b>\n\nВведите номер телефона в международном формате:",
        reply_markup=cancel_menu_keyboard(),
        parse_mode="HTML",
    )


@router.message(SendSmsFSM.waiting_phone)
async def process_sms_phone(message: types.Message, state: FSMContext):
    text = await _require_text(message)
    if text is None:
        return
    if text in (BTN_BACK, BTN_MENU):
        await go_to_main_menu(message, state)
        return

    phone = text
    if not phone.startswith("+"):
        await message.answer(
            "❌ Номер должен начинаться с '+' и кода страны.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    country_code = detect_country_code(phone)
    await state.update_data(phone=phone, country_code=country_code)
    await state.set_state(SendSmsFSM.waiting_choose)

    if country_code:
        answer_text = (
            f"📍 Определена страна: <b>{country_code}</b>\n\n"
            "Выберите способ создания сообщения:"
        )
    else:
        answer_text = (
            "⚠️ Не удалось определить страну по номеру.\n"
            "Можно только набрать сообщение вручную."
        )

    await message.answer(
        answer_text,
        reply_markup=_choose_keyboard(country_code),
        parse_mode="HTML",
    )


@router.message(SendSmsFSM.waiting_choose)
async def process_sms_choose(message: types.Message, state: FSMContext):
    text = await _require_text(message)
    if text is None:
        return
    if text == BTN_MENU:
        await go_to_main_menu(message, state)
        return
    if text == BTN_BACK:
        await state.set_state(SendSmsFSM.waiting_phone)
        await message.answer(
            "Введите номер телефона в международном формате:",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    if text == BTN_MANUAL:
        await state.set_state(SendSmsFSM.waiting_text)
        await message.answer(
            "Введите текст сообщения.\n\n"
            "Используйте <code>{link}</code> там, где должна быть короткая ссылка.\n"
            "Например: <code>Привет! Вот твоя ссылка: {link}</code>",
            reply_markup=cancel_menu_keyboard(),
            parse_mode="HTML",
        )
        return

    if text == BTN_TEMPLATE:
        data = await state.get_data()
        country_code = data.get("country_code")
        if not country_code:
            await message.answer(
                "❌ Страна не определена. Наберите сообщение вручную.",
                reply_markup=cancel_menu_keyboard(),
            )
            return

        try:
            templates = await api.list_templates()
        except Exception as e:
            await message.answer(f"❌ Ошибка получения шаблонов: {e}")
            return

        country_templates = [t for t in templates if t["country_code"] == country_code]
        if not country_templates:
            await message.answer(
                f"📭 Нет шаблонов для страны <b>{country_code}</b>.\n"
                "Сначала добавьте шаблон в разделе <b>Шаблоны</b> или наберите сообщение вручную.",
                reply_markup=cancel_menu_keyboard(),
                parse_mode="HTML",
            )
            return

        builder = InlineKeyboardBuilder()
        for t in country_templates:
            builder.button(
                text=t["name"] or "Без названия",
                callback_data=f"sms_templ:{t['id']}",
            )
        builder.adjust(1)

        lines = [f"📋 <b>Шаблоны для {country_code}:</b>"]
        for t in country_templates:
            star = " ⭐" if t.get("is_favorite") else ""
            name = t["name"] or "Без названия"
            lines.append(f"{star}<b>{name}</b>\n<code>{t['text']}</code>")

        await message.answer(
            "\n\n".join(lines),
            reply_markup=builder.as_markup(),
            parse_mode="HTML",
        )
        return

    await message.answer(
        "❌ Выберите один из вариантов: «Набрать вручную» или «Выбрать шаблон».",
        reply_markup=_choose_keyboard((await state.get_data()).get("country_code")),
    )


@router.callback_query(F.data.startswith("sms_templ:"))
async def cb_sms_template_selected(query: types.CallbackQuery, state: FSMContext):
    template_id = query.data.split(":")[-1]
    try:
        templates = await api.list_templates()
    except Exception as e:
        await query.answer(f"Ошибка: {e}", show_alert=True)
        return

    template = next((t for t in templates if str(t["id"]) == template_id), None)
    if not template:
        await query.answer("Шаблон не найден", show_alert=True)
        return

    await state.update_data(
        text=template["text"],
        template_name=template["name"] or "Без названия",
    )
    await state.set_state(SendSmsFSM.waiting_url)

    name = template["name"] or "Без названия"
    await query.message.answer(
        f"✅ Выбран шаблон: <b>{name}</b>\n\n"
        f"<code>{template['text']}</code>\n\n"
        "Введите целевую ссылку (URL), на которую будет вести <code>{link}</code>:",
        reply_markup=cancel_menu_keyboard(),
        parse_mode="HTML",
    )
    await query.answer()


@router.message(SendSmsFSM.waiting_text)
async def process_sms_text(message: types.Message, state: FSMContext):
    text = await _require_text(message)
    if text is None:
        return
    if text == BTN_MENU:
        await go_to_main_menu(message, state)
        return
    if text == BTN_BACK:
        await state.set_state(SendSmsFSM.waiting_choose)
        data = await state.get_data()
        await message.answer(
            "Выберите способ создания сообщения:",
            reply_markup=_choose_keyboard(data.get("country_code")),
        )
        return

    if not text:
        await message.answer(
            "❌ Текст не может быть пустым.",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    if "{link}" not in text:
        await message.answer(
            "❌ В сообщении должен быть плейсхолдер <code>{link}</code>.\n"
            "Он будет заменён на короткую ссылку.",
            reply_markup=cancel_menu_keyboard(),
            parse_mode="HTML",
        )
        return

    await state.update_data(text=text)
    await state.set_state(SendSmsFSM.waiting_url)
    await message.answer(
        "Введите целевую ссылку (URL), на которую будет вести <code>{link}</code>:",
        reply_markup=cancel_menu_keyboard(),
        parse_mode="HTML",
    )


@router.message(SendSmsFSM.waiting_url)
async def process_sms_url(message: types.Message, state: FSMContext):
    text = await _require_text(message)
    if text is None:
        return
    if text == BTN_MENU:
        await go_to_main_menu(message, state)
        return
    if text == BTN_BACK:
        data = await state.get_data()
        if data.get("template_name"):
            await state.set_state(SendSmsFSM.waiting_choose)
            await message.answer(
                "Выберите способ создания сообщения:",
                reply_markup=_choose_keyboard(data.get("country_code")),
            )
        else:
            await state.set_state(SendSmsFSM.waiting_text)
            await message.answer(
                "Введите текст сообщения с плейсхолдером <code>{link}</code>:",
                reply_markup=cancel_menu_keyboard(),
                parse_mode="HTML",
            )
        return

    url = text
    if not url.startswith(("http://", "https://")):
        await message.answer(
            "❌ Ссылка должна начинаться с http:// или https://",
            reply_markup=cancel_menu_keyboard(),
        )
        return

    await state.update_data(url=url)
    await state.set_state(SendSmsFSM.waiting_sender_id)
    await message.answer(
        "Введите имя отправителя (Sender ID).\n\n"
        "Максимум 11 латинских букв/цифр. Нажмите <b>По умолчанию (TRACKING)</b>, "
        "чтобы оставить стандартное значение.",
        reply_markup=_sender_id_keyboard(),
        parse_mode="HTML",
    )


@router.message(SendSmsFSM.waiting_sender_id)
async def process_sms_sender_id(message: types.Message, state: FSMContext):
    text = await _require_text(message)
    if text is None:
        return
    if text == BTN_MENU:
        await go_to_main_menu(message, state)
        return
    if text == BTN_BACK:
        await state.set_state(SendSmsFSM.waiting_url)
        await message.answer(
            "Введите целевую ссылку (URL), на которую будет вести <code>{link}</code>:",
            reply_markup=cancel_menu_keyboard(),
            parse_mode="HTML",
        )
        return

    sender_id = "TRACKING" if text == BTN_DEFAULT_SENDER else text.strip()
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

    data = await _get_state_data_or_cancel(message, state, "phone", "text", "url")
    if data is None:
        return

    phone = data["phone"]
    message_text = data["text"]
    url = data["url"]
    template_name = data.get("template_name")

    await message.answer("⏳ Отправка SMS...")

    try:
        result = await api.send_sms(
            phone,
            message_text,
            message.from_user.id,
            url=url,
            template_name=template_name,
            sender_id=sender_id,
        )
    except Exception as e:
        await state.clear()
        is_owner = await user_is_owner(message.from_user.id)
        await message.answer(
            f"❌ Ошибка отправки SMS: {e}",
            reply_markup=main_menu_keyboard(is_owner=is_owner),
        )
        return

    await state.clear()

    status = "✅" if result.get("success") else "❌"
    short_link = result.get("short_link")
    campaign_id = result.get("campaign_id")

    answer_text = f"{status} <b>Статус:</b> {result.get('message')}\n"
    if short_link:
        answer_text += f"<b>Короткая ссылка:</b> {short_link}\n"
    if campaign_id:
        answer_text += f"<b>ID кампании:</b> <code>{campaign_id}</code>\n"
    answer_text += f"<b>Ответ шлюза:</b> <code>{result.get('provider_response')}</code>"

    is_owner = await user_is_owner(message.from_user.id)
    await message.answer(
        answer_text,
        reply_markup=main_menu_keyboard(is_owner=is_owner),
        parse_mode="HTML",
    )
