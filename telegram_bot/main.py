import asyncio
import logging
import signal
import sys

from aiogram import Bot, Dispatcher
from aiogram.client.default import DefaultBotProperties
from aiogram.fsm.storage.memory import MemoryStorage
from aiogram.enums import ParseMode
from aiogram.types import BotCommand
from aiogram.webhook.aiohttp_server import SimpleRequestHandler, setup_application
from aiohttp import web

from config import settings
from handlers import admin, campaigns, common, internal, keys, settings as settings_handler, sms, stats, templates
from middlewares.auth import AdminMiddleware
from services.api import api

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(name)s - %(message)s",
    stream=sys.stdout,
)
logger = logging.getLogger(__name__)


def setup_routers(dp: Dispatcher) -> None:
    routers = [
        common.router,
        admin.router,
        keys.router,
        sms.router,
        stats.router,
        templates.router,
        campaigns.router,
        settings_handler.router,
    ]

    for router in routers:
        router.message.middleware(AdminMiddleware())
        router.callback_query.middleware(AdminMiddleware())

    dp.include_routers(*routers)


async def set_bot_commands(bot: Bot) -> None:
    commands = [
        BotCommand(command="start", description="Начать работу"),
        BotCommand(command="help", description="Справка"),
    ]
    await bot.set_my_commands(commands)


async def verify_owner() -> bool:
    """Проверяем, что OWNER_TELEGRAM_ID указан и является владельцем в backend.

    Если владельца ещё нет в базе, автоматически создаём его.
    """
    try:
        owner = await api.ensure_owner(
            settings.owner_telegram_id,
            getattr(settings, "owner_username", None),
        )
        return bool(owner.get("is_owner"))
    except Exception as e:
        logger.error("Failed to verify/ensure owner: %s", e)
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


def create_web_app(bot: Bot, dp: Dispatcher) -> web.Application:
    app = web.Application()
    app["bot"] = bot
    app.router.add_post("/internal/notify", internal.notify_handler)
    return app


async def run_internal_server(app: web.Application) -> None:
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(
        runner, host=settings.webhook_host, port=settings.webhook_port
    )
    await site.start()
    logger.info(
        "Internal server started on %s:%s", settings.webhook_host, settings.webhook_port
    )

    stop_event = asyncio.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        try:
            asyncio.get_event_loop().add_signal_handler(sig, stop_event.set)
        except NotImplementedError:
            pass
    await stop_event.wait()
    await runner.cleanup()


async def run_polling() -> None:
    bot = Bot(
        token=settings.bot_token,
        default=DefaultBotProperties(parse_mode=ParseMode.HTML),
    )
    dp = Dispatcher(storage=MemoryStorage())
    dp.startup.register(on_startup)
    dp.shutdown.register(on_shutdown)
    setup_routers(dp)

    app = create_web_app(bot, dp)

    await asyncio.gather(
        dp.start_polling(bot),
        run_internal_server(app),
    )


async def run_webhook() -> None:
    if not settings.webhook_url:
        raise ValueError("WEBHOOK_URL must be set when BOT_MODE=webhook")

    bot = Bot(
        token=settings.bot_token,
        default=DefaultBotProperties(parse_mode=ParseMode.HTML),
    )
    dp = Dispatcher(storage=MemoryStorage())
    dp.startup.register(on_startup)
    dp.shutdown.register(on_shutdown)
    setup_routers(dp)

    await bot.set_webhook(
        url=f"{settings.webhook_url}{settings.webhook_path}",
        drop_pending_updates=True,
    )

    app = create_web_app(bot, dp)
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

    stop_event = asyncio.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        try:
            asyncio.get_event_loop().add_signal_handler(sig, stop_event.set)
        except NotImplementedError:
            pass
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
