import asyncio
import logging

from aiohttp import web
from aiogram import Bot

from config import settings


logger = logging.getLogger(__name__)


async def notify_handler(request: web.Request) -> web.Response:
    token = request.headers.get("X-Internal-Bot-Token")
    if token != settings.internal_bot_token:
        return web.Response(status=401, text="Unauthorized")

    try:
        data = await request.json()
    except Exception:
        return web.Response(status=400, text="Bad request")

    telegram_id = data.get("telegram_id")
    text = data.get("text")
    if not telegram_id or not text:
        return web.Response(status=400, text="Missing telegram_id or text")

    bot: Bot = request.app["bot"]
    try:
        # Таймаут на отправку, чтобы не блокировать internal server,
        # если Telegram API недоступен или долго отвечает.
        await asyncio.wait_for(
            bot.send_message(telegram_id, text, parse_mode="HTML"),
            timeout=15.0,
        )
    except asyncio.TimeoutError:
        logger.error("Timeout sending notification to telegram_id=%s", telegram_id)
        return web.Response(status=504, text="Failed to send: timeout")
    except Exception as e:
        logger.exception("Failed to send notification to telegram_id=%s: %s", telegram_id, e)
        return web.Response(status=502, text=f"Failed to send: {e}")

    logger.info("Notification sent to telegram_id=%s", telegram_id)
    return web.Response(status=204)
