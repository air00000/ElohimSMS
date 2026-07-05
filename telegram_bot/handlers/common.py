from aiogram import Router
from aiogram.filters import Command
from aiogram.types import Message

router = Router()


@router.message(Command("start"))
async def cmd_start(message: Message):
    await message.answer(
        "👋 Добро пожаловать в ElohimSMS Bot.\n\n"
        "Этот бот предназначен для администрирования сервиса рассылки SMS.\n"
        "Используйте /help для просмотра доступных команд."
    )


@router.message(Command("help"))
async def cmd_help(message: Message):
    await message.answer(
        "📋 Доступные команды:\n\n"
        "<b>Администрирование:</b>\n"
        "/add_admin &lt;telegram_id&gt; [username] — назначить администратора\n"
        "/remove_admin &lt;telegram_id&gt; — снять права администратора\n"
        "/list_admins — список администраторов\n\n"
        "<b>API-ключи:</b>\n"
        "/create_key &lt;название&gt; — создать новый API-ключ\n"
        "/list_keys — список API-ключей\n"
        "/revoke_key &lt;id&gt; — отозвать API-ключ\n\n"
        "<b>SMS-шаблоны по странам:</b>\n"
        "/set_template &lt;код_страны&gt; &lt;текст&gt; — сохранить шаблон\n"
        "/templates — список шаблонов\n"
        "/delete_template &lt;код_страны&gt; — удалить шаблон\n\n"
        "<b>Кампании:</b>\n"
        "/send_campaign &lt;номер&gt; &lt;ссылка&gt; — отправить фишинг-кампанию\n"
        "/send_sms &lt;номер&gt; &lt;текст&gt; — отправить обычное SMS\n\n"
        "<b>Статистика:</b>\n"
        "/stats — статистика сервиса\n\n"
        "Placeholders в шаблонах: {link}, {phone}, {country}",
        parse_mode="HTML",
    )
