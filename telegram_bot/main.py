import asyncio
import logging
import signal
import sys

from aiogram import Bot, Dispatcher
from aiogram.client.default import DefaultBotProperties
from aiogram.enums import ParseMode
from aiogram.types import BotCommand
from aiogram.webhook.aiohttp_server import SimpleRequestHandler, setup_application
from aiohttp import web

from config import settings
from handlers import admin, campaigns, common, keys, sms, stats, templates
from middlewares.auth import AdminMiddleware
from services.api import api

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(name)s - %(message)s",
    stream=sys.stdout,
)
logger = logging.getLogger(__name__)


def setup_routers(dp: Dispatcher) -> None:
    for router in [
        admin.router,
        keys.router,
        sms.router,
        stats.router,
        templates.router,
        campaigns.router,
    ]:
        router.message.middleware(AdminMiddleware())

    dp.include_routers(
        common.router,
        admin.router,
        keys.router,
        sms.router,
        stats.router,
        templates.router,
        campaigns.router,
    )


async def set_bot_commands(bot: Bot) -> None:
    commands = [
        BotCommand(command="start", description="Начать работу"),
        BotCommand(command="help", description="Справка по командам"),
        BotCommand(command="list_admins", description="Список администраторов"),
        BotCommand(command="add_admin", description="Добавить администратора"),
        BotCommand(command="remove_admin", description="Удалить администратора"),
        BotCommand(command="list_keys", description="Список API-ключей"),
        BotCommand(command="create_key", description="Создать API-ключ"),
        BotCommand(command="revoke_key", description="Отозвать API-ключ"),
        BotCommand(command="templates", description="Список SMS-шаблонов"),
        BotCommand(command="set_template", description="Добавить/изменить шаблон"),
        BotCommand(command="delete_template", description="Удалить шаблон"),
        BotCommand(command="send_campaign", description="Отправить тестовую кампанию"),
        BotCommand(command="send_sms", description="Отправить тестовое SMS"),
        BotCommand(command="stats", description="Статистика сервиса"),
    ]
    await bot.set_my_commands(commands)


async def verify_owner() -> bool:
    """Проверяем, что OWNER_TELEGRAM_ID указан и является владельцем в backend."""
    try:
        admins = await api.list_admins()
        for admin in admins:
            if admin["telegram_id"] == settings.owner_telegram_id and admin.get(
                "is_owner"
            ):
                return True
        logger.warning(
            "OWNER_TELEGRAM_ID %s is not registered as owner in backend",
            settings.owner_telegram_id,
        )
        return False
    except Exception as e:
        logger.error("Failed to verify owner: %s", e)
        return False


async def on_startup(bot: Bot) -> None:
    logger.info("Starting ElohimSMS Telegram bot")
    await set_bot_commands(bot)
    if await verify_owner():
        logger.info("Owner verified successfully")
    else:
        logger.warning("Owner verification failed — some commands may not work")


async def on_shutdown(bot: Bot) -> None:
    logger.info("Shutting down ElohimSMS Telegram bot")
    await bot.session.close()


async def run_polling() -> None:
    bot = Bot(
        token=settings.bot_token,
        default=DefaultBotProperties(parse_mode=ParseMode.HTML),
    )
    dp = Dispatcher()
    dp.startup.register(on_startup)
    dp.shutdown.register(on_shutdown)
    setup_routers(dp)

    await dp.start_polling(bot)


async def run_webhook() -> None:
    if not settings.webhook_url:
        raise ValueError("WEBHOOK_URL must be set when BOT_MODE=webhook")

    bot = Bot(
        token=settings.bot_token,
        default=DefaultBotProperties(parse_mode=ParseMode.HTML),
    )
    dp = Dispatcher()
    dp.startup.register(on_startup)
    dp.shutdown.register(on_shutdown)
    setup_routers(dp)

    await bot.set_webhook(
        url=f"{settings.webhook_url}{settings.webhook_path}",
        drop_pending_updates=True,
    )

    app = web.Application()
    webhook_handler = SimpleRequestHandler(dispatcher=dp, bot=bot)
    webhook_handler.register(app, path=settings.webhook_path)
    setup_application(app, dp, bot=bot)

    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, host=settings.webhook_host, port=settings.webhook_port)
    await site.start()

    logger.info(
        "Webhook server started on %s:%s", settings.webhook_host, settings.webhook_port
    )

    # Graceful shutdown
    stop_event = asyncio.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        asyncio.get_event_loop().add_signal_handler(sig, stop_event.set)
    await stop_event.wait()

    await runner.cleanup()
    await bot.delete_webhook(drop_pending_updates=True)


def main() -> None:
    if settings.bot_mode.lower() == "webhook":
        asyncio.run(run_webhook())
    else:
        asyncio.run(run_polling())


if __name__ == "__main__":
    main()
