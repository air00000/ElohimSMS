import asyncio
from typing import Any

import aiohttp
from aiohttp import ClientError, ClientResponseError

from config import settings


class BackendAPI:
    def __init__(self):
        self.base_url = settings.api_base_url.rstrip("/")
        self.headers = {
            "X-Internal-Bot-Token": settings.internal_bot_token,
            "Content-Type": "application/json",
        }

    async def request(
        self,
        method: str,
        path: str,
        json: dict | None = None,
        params: dict | None = None,
        retries: int = 3,
        backoff: float = 1.0,
    ) -> dict[str, Any]:
        url = f"{self.base_url}{path}"
        last_error = None

        for attempt in range(retries):
            try:
                async with aiohttp.ClientSession() as session:
                    async with session.request(
                        method, url, headers=self.headers, json=json, params=params
                    ) as response:
                        data = await response.json()
                        if not response.ok:
                            error_msg = (
                                data.get("error", data)
                                if isinstance(data, dict)
                                else data
                            )
                            raise ClientResponseError(
                                response.request_info,
                                response.history,
                                status=response.status,
                                message=str(error_msg),
                            )
                        return data
            except (ClientError, TimeoutError) as e:
                last_error = e
                if attempt < retries - 1:
                    wait = backoff * (2**attempt)
                    await asyncio.sleep(wait)
                continue

        raise RuntimeError(f"API request failed after {retries} attempts: {last_error}")

    async def health(self) -> dict[str, Any]:
        return await self.request("GET", "/health", retries=1)

    # ---------- Администраторы ----------

    async def list_admins(self) -> list[dict[str, Any]]:
        result = await self.request("GET", "/bot/v1/admin")
        return result.get("data", []) if isinstance(result, dict) else result

    async def create_admin(
        self, telegram_id: int, username: str | None
    ) -> dict[str, Any]:
        return await self.request(
            "POST",
            "/bot/v1/admin",
            json={"telegram_id": telegram_id, "username": username},
        )

    async def ensure_owner(
        self, telegram_id: int, username: str | None
    ) -> dict[str, Any]:
        return await self.request(
            "POST",
            "/bot/v1/admin/ensure_owner",
            json={"telegram_id": telegram_id, "username": username},
        )

    async def remove_admin(self, telegram_id: int) -> None:
        await self.request("DELETE", f"/bot/v1/admin/{telegram_id}")

    async def update_sender_name(
        self, telegram_id: int, sender_name: str | None
    ) -> dict[str, Any]:
        return await self.request(
            "POST",
            "/bot/v1/admin/me/sender_name",
            json={"telegram_id": telegram_id, "sender_name": sender_name},
        )

    # ---------- API-ключи ----------

    async def list_keys(self) -> list[dict[str, Any]]:
        result = await self.request("GET", "/bot/v1/keys")
        return result.get("data", []) if isinstance(result, dict) else result

    async def create_key(self, name: str, created_by: int) -> dict[str, Any]:
        return await self.request(
            "POST",
            "/bot/v1/keys",
            json={"name": name, "created_by_telegram_id": created_by},
        )

    async def revoke_key(self, key_id: str) -> None:
        await self.request("POST", f"/bot/v1/keys/{key_id}/revoke")

    # ---------- Шаблоны ----------

    async def list_templates(self) -> list[dict[str, Any]]:
        result = await self.request("GET", "/bot/v1/templates")
        return result.get("data", []) if isinstance(result, dict) else result

    async def create_template(
        self, country_code: str, text: str, name: str
    ) -> dict[str, Any]:
        return await self.request(
            "POST",
            "/bot/v1/templates",
            json={
                "country_code": country_code,
                "text": text,
                "name": name,
            },
        )

    async def delete_template(self, template_id: str) -> None:
        await self.request("DELETE", f"/bot/v1/templates/{template_id}")

    async def set_favorite_template(self, template_id: str) -> dict[str, Any]:
        return await self.request(
            "POST", f"/bot/v1/templates/{template_id}/favorite"
        )

    # ---------- SMS / Кампании ----------

    async def send_campaign(
        self, phone: str, url: str, telegram_id: int
    ) -> dict[str, Any]:
        return await self.request(
            "POST",
            "/bot/v1/campaigns/send",
            json={
                "phone": phone,
                "url": url,
                "telegram_id": telegram_id,
            },
        )

    async def send_sms(
        self,
        phone: str,
        message: str,
        telegram_id: int,
        url: str | None = None,
        template_name: str | None = None,
    ) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "phone": phone,
            "message": message,
            "telegram_id": telegram_id,
        }
        if url is not None:
            payload["url"] = url
        if template_name is not None:
            payload["template_name"] = template_name
        return await self.request(
            "POST",
            "/bot/v1/sms/send",
            json=payload,
        )

    # ---------- Статистика ----------

    async def get_stats(self) -> dict[str, Any]:
        return await self.request("GET", "/bot/v1/stats")


api = BackendAPI()
