from aiohttp import web
from aiogram import Bot

from config import settings


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
        await bot.send_message(telegram_id, text, parse_mode="HTML")
    except Exception as e:
        return web.Response(status=502, text=f"Failed to send: {e}")

    return web.Response(status=204)
