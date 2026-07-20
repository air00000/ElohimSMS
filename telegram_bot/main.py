import asyncio
import logging
import signal
import sys
import threading

from aiogram import Bot, Dispatcher
from aiogram.client.default import DefaultBotProperties
from aiogram.fsm.storage.memory import MemoryStorage
from aiogram.enums import ParseMode
from aiogram.types import BotCommand
from aiogram.webhook.aiohttp_server import SimpleRequestHandler, setup_application
from aiohttp import web

from config import settings
from handlers import admin, common, internal, keys, settings as settings_handler, sms, stats, templates
from middlewares.auth import AdminMiddleware
from services.api import api

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(name)s - %(message)s",
    stream=sys.stdout,
)
logger = logging.getLogger(__name__)


def setup_routers(dp: Dispatcher) -> None:
    # common.router — публичные команды /start, /help, /menu и кнопки назад/меню.
    # Остальные роутеры требуют прав администратора.
    admin_routers = [
        admin.router,
        keys.router,
        sms.router,
        stats.router,
        templates.router,
        settings_handler.router,
    ]

    for router in admin_routers:
        router.message.middleware(AdminMiddleware())
        router.callback_query.middleware(AdminMiddleware())

    dp.include_routers(*admin_routers, common.router)


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
    await api.close()
    await bot.session.close()


def create_web_app(bot: Bot, dp: Dispatcher | None = None) -> web.Application:
    app = web.Application()
    app["bot"] = bot
    app.router.add_post("/internal/notify", internal.notify_handler)
    return app


def run_internal_server_thread(
    stop_event: threading.Event,
) -> None:
    """Запускает internal aiohttp сервер в отдельном потоке.

    aiogram polling и aiohttp server не должны делить один event loop,
    иначе polling монополизирует loop и HTTP-запросы к /internal/notify
    будут таймаутиться.

    Для internal server создаём отдельный экземпляр Bot, чтобы не делить
    один aiohttp session между разными event loop / потоками.
    """
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    runner = None
    bot = None

    async def _serve() -> None:
        nonlocal runner, bot
        bot = Bot(
            token=settings.bot_token,
            default=DefaultBotProperties(parse_mode=ParseMode.HTML),
        )
        app = create_web_app(bot)
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(
            runner, host=settings.webhook_host, port=settings.webhook_port
        )
        await site.start()
        logger.info(
            "Internal server started on %s:%s",
            settings.webhook_host,
            settings.webhook_port,
        )
        while not stop_event.is_set():
            await asyncio.sleep(0.5)
        await runner.cleanup()

    try:
        loop.run_until_complete(_serve())
    except Exception as exc:
        logger.exception("Internal server error: %s", exc)
    finally:
        try:
            if runner is not None:
                loop.run_until_complete(runner.cleanup())
        except Exception as exc:
            logger.warning("Failed to cleanup internal server runner: %s", exc)
        try:
            if bot is not None:
                loop.run_until_complete(bot.session.close())
        except Exception as exc:
            logger.warning("Failed to close internal bot session: %s", exc)
        loop.close()


async def run_polling() -> None:
    bot = Bot(
        token=settings.bot_token,
        default=DefaultBotProperties(parse_mode=ParseMode.HTML),
    )
    dp = Dispatcher(storage=MemoryStorage())
    dp.startup.register(on_startup)
    dp.shutdown.register(on_shutdown)
    setup_routers(dp)

    internal_stop_event = threading.Event()
    polling_stop_event = asyncio.Event()

    def _signal_handler() -> None:
        internal_stop_event.set()
        polling_stop_event.set()

    for sig in (signal.SIGINT, signal.SIGTERM):
        try:
            asyncio.get_event_loop().add_signal_handler(sig, _signal_handler)
        except NotImplementedError:
            pass

    server_thread = threading.Thread(
        target=run_internal_server_thread,
        args=(internal_stop_event,),
        daemon=True,
    )
    server_thread.start()

    polling_task = asyncio.create_task(dp.start_polling(bot))

    await polling_stop_event.wait()

    polling_task.cancel()
    try:
        await polling_task
    except asyncio.CancelledError:
        pass

    internal_stop_event.set()
    server_thread.join(timeout=5.0)
    if server_thread.is_alive():
        logger.warning("Internal server thread did not stop gracefully")


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

    await bot.set_webhook(
        url=f"{settings.webhook_url}{settings.webhook_path}",
        drop_pending_updates=True,
    )

    stop_event = asyncio.Event()
    for sig in (signal.SIGINT, signal.SIGTERM):
        try:
            asyncio.get_event_loop().add_signal_handler(sig, stop_event.set)
        except NotImplementedError:
            pass
    await stop_event.wait()

    await bot.delete_webhook(drop_pending_updates=True)
    await runner.cleanup()


def main() -> None:
    mode = settings.bot_mode.lower()
    if mode not in ("polling", "webhook"):
        raise ValueError(f"Unknown BOT_MODE: {settings.bot_mode!r}")
    if mode == "webhook":
        asyncio.run(run_webhook())
    else:
        asyncio.run(run_polling())


if __name__ == "__main__":
    main()
