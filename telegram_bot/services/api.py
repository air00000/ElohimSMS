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

    async def remove_admin(self, telegram_id: int) -> None:
        await self.request("GET", f"/bot/v1/admin/{telegram_id}")

    async def list_keys(self, page: int = 1, per_page: int = 20) -> dict[str, Any]:
        return await self.request(
            "GET", "/bot/v1/keys", params={"page": page, "per_page": per_page}
        )

    async def create_key(self, name: str, created_by: int) -> dict[str, Any]:
        return await self.request(
            "POST",
            "/bot/v1/keys",
            json={"name": name, "created_by_telegram_id": created_by},
        )

    async def revoke_key(self, key_id: str) -> None:
        await self.request("POST", f"/bot/v1/keys/{key_id}/revoke")

    async def list_templates(self) -> list[dict[str, Any]]:
        return await self.request("GET", "/bot/v1/templates")

    async def create_or_update_template(
        self, country_code: str, text: str
    ) -> dict[str, Any]:
        return await self.request(
            "POST",
            "/bot/v1/templates",
            json={"country_code": country_code, "text": text},
        )

    async def delete_template(self, country_code: str) -> None:
        await self.request("GET", f"/bot/v1/templates/{country_code}")

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

    async def get_balance(self) -> dict[str, Any]:
        return await self.request("GET", "/api/v1/sms/balance")

    async def get_routes(self) -> dict[str, Any]:
        return await self.request("GET", "/api/v1/sms/routes")

    async def get_stats(self) -> dict[str, Any]:
        keys = await self.list_keys(per_page=1)
        admins = await self.list_admins()
        balance = await self.get_balance()
        return {
            "admins_count": len(admins),
            "keys_total": keys.get("total", 0),
            "keys_active": sum(1 for k in keys.get("data", []) if k.get("is_active")),
            "balance": balance.get("balance", "N/A"),
        }


api = BackendAPI()
