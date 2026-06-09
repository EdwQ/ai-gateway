import json
import time
import uuid
from collections.abc import AsyncGenerator
from decimal import Decimal
from typing import Any, Optional

import httpx
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.config import get_settings
from app.core.security import decrypt_value
from app.models.alias import ModelAlias
from app.models.provider import Provider, ProviderKey
from app.models.usage import UsageLog
from app.models.user import User

settings = get_settings()

# Model pricing: $/1M tokens (input, output)
MODEL_PRICES: dict[str, tuple[float, float]] = {
    "gpt-4o": (2.50, 10.00),
    "gpt-4o-mini": (0.15, 0.60),
    "gpt-4": (30.00, 60.00),
    "gpt-4-turbo": (10.00, 30.00),
    "gpt-3.5-turbo": (0.50, 1.50),
    "claude-3-5-sonnet": (3.00, 15.00),
    "claude-3-5-haiku": (0.80, 4.00),
    "claude-3-opus": (15.00, 75.00),
    "gemini-2.0-flash": (0.10, 0.40),
    "gemini-2.0-pro": (2.00, 8.00),
    "deepseek-chat": (0.14, 0.28),
    "deepseek-reasoner": (0.55, 2.19),
    "qwen-max": (1.60, 4.80),
    "qwen-plus": (0.40, 1.20),
    "qwen-turbo": (0.15, 0.60),
}

USD_TO_RMB = 7.25


def calculate_cost(model: str, prompt_tokens: int, completion_tokens: int) -> Decimal:
    """Calculate RMB cost based on model pricing."""
    model_key = model.lower()
    # Try exact match first, then prefix match
    price = MODEL_PRICES.get(model_key)
    if not price:
        for key, val in MODEL_PRICES.items():
            if model_key.startswith(key) or key.startswith(model_key):
                price = val
                break
    if not price:
        # Default pricing if model not found
        price = (1.0, 3.0)

    input_price_per_m, output_price_per_m = price
    cost_usd = (prompt_tokens / 1_000_000 * input_price_per_m) + (
        completion_tokens / 1_000_000 * output_price_per_m
    )
    return Decimal(str(round(cost_usd * USD_TO_RMB, 6)))


class GatewayService:
    """AI Gateway proxy service."""

    def __init__(self):
        self._key_index: dict[str, int] = {}  # provider_id -> current index

    async def _get_provider(self, db: AsyncSession, model: str) -> Optional[Provider]:
        """Find best provider for the given model."""
        result = await db.execute(
            select(Provider)
            .where(
                Provider.is_active == True,  # noqa: E712
                Provider.health_status.in_(["unknown", "healthy", "degraded"]),
            )
            .order_by(Provider.priority)
        )
        providers = result.scalars().all()

        for provider in providers:
            models = provider.models or []
            if model in models:
                return provider
            # Check if any model in provider matches (wildcard)
            for pm in models:
                if pm.endswith("*") and model.startswith(pm.rstrip("*")):
                    return provider
                if model.startswith(pm):
                    return provider

        # Return first active provider if no specific match
        return providers[0] if providers else None

    async def _get_next_key(
        self, db: AsyncSession, provider_id: str
    ) -> Optional[tuple[ProviderKey, Provider]]:
        """Get next available key using round-robin with failover.
        
        Falls back to provider.api_key_encrypted if no ProviderKey records exist.
        """
        result = await db.execute(
            select(ProviderKey, Provider)
            .join(Provider, ProviderKey.provider_id == Provider.id)
            .where(
                ProviderKey.provider_id == provider_id,
                ProviderKey.is_active == True,  # noqa: E712
            )
        )
        keys = result.all()

        if not keys:
            # Fallback: use provider.api_key_encrypted directly
            result = await db.execute(
                select(Provider).where(Provider.id == provider_id)
            )
            provider = result.scalar_one_or_none()
            if provider and provider.api_key_encrypted:
                # Create a transient ProviderKey object (not added to session)
                virtual_key = ProviderKey(
                    id=uuid.uuid4(),
                    provider_id=provider.id,
                    key_encrypted=provider.api_key_encrypted,
                    is_active=True,
                    weight=1,
                    fail_count=0,
                    max_fail_count=3,
                )
                return virtual_key, provider
            return None

        # Filter out failed keys
        available = [
            (k, p)
            for k, p in keys
            if k.fail_count < k.max_fail_count
        ]
        if not available:
            # Reset fail counts if all keys are down
            for k, p in keys:
                k.fail_count = 0
            await db.flush()
            available = [(k, p) for k, p in keys]

        # Simple round-robin
        idx = self._key_index.get(str(provider_id), 0) % len(available)
        self._key_index[str(provider_id)] = idx + 1
        return available[idx]

    async def _resolve_alias(
        self, db: AsyncSession, model: str
    ) -> str:
        """Resolve a model alias to the real model name.
        
        If the model name matches an alias, returns the target model.
        Otherwise returns the original model name.
        """
        result = await db.execute(
            select(ModelAlias).where(
                ModelAlias.alias_name == model,
                ModelAlias.is_active == True,  # noqa: E712
            )
        )
        alias = result.scalar_one_or_none()
        return alias.target_model if alias else model

    async def _check_user_allowed(
        self, user: User, model: str, db: AsyncSession
    ) -> None:
        """Check if user is allowed to use this model (alias)."""
        if user.role in ("admin", "super_admin", "finance"):
            return  # Admins can use any model
        allowed = user.allowed_models or []
        if allowed and model not in allowed:
            raise ValueError(
                f"Model '{model}' is not in your allowed models list. "
                f"Allowed: {', '.join(allowed)}"
            )

    async def chat_completion(
        self,
        db: AsyncSession,
        user: User,
        model: str,
        messages: list[dict],
        stream: bool = False,
        **kwargs,
    ) -> dict | AsyncGenerator[bytes, None]:
        """Proxy chat completion to provider."""
        # Step 1: Check user's allowed models
        await self._check_user_allowed(user, model, db)

        # Step 2: Resolve alias -> real model name
        real_model = await self._resolve_alias(db, model)

        # Step 3: Find provider for the real model
        provider = await self._get_provider(db, real_model)
        if not provider:
            raise ValueError(f"No active provider found for model: {real_model}")

        key_result = await self._get_next_key(db, str(provider.id))
        if not key_result:
            raise ValueError(f"No available API keys for provider: {provider.name}")

        provider_key, provider = key_result
        api_key = decrypt_value(provider_key.key_encrypted, settings.ENCRYPTION_KEY)
        base_url = provider.base_url.rstrip("/")

        headers = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        }

        body = {
            "model": real_model,
            "messages": messages,
            "stream": stream,
        }
        # Add optional params
        for param in ["temperature", "max_tokens", "top_p", "frequency_penalty",
                       "presence_penalty", "stop"]:
            if param in kwargs and kwargs[param] is not None:
                body[param] = kwargs[param]

        request_id = str(uuid.uuid4())
        start_time = time.time()

        try:
            async with httpx.AsyncClient(timeout=120.0) as client:
                if stream:
                    return self._handle_stream(
                        client, provider, provider_key, db, user,
                        model, request_id, base_url, headers, body, start_time,
                    )
                else:
                    return await self._handle_non_stream(
                        client, provider, provider_key, db, user,
                        model, request_id, base_url, headers, body, start_time,
                    )
        except Exception as e:
            # Record failure
            provider_key.fail_count += 1
            await db.flush()
            raise

    async def _handle_non_stream(
        self, client, provider, provider_key, db, user,
        model, request_id, base_url, headers, body, start_time,
    ) -> dict:
        """Handle non-streaming response."""
        resp = await client.post(
            f"{base_url}/v1/chat/completions",
            headers=headers,
            json=body,
        )
        duration = int((time.time() - start_time) * 1000)

        if resp.status_code != 200:
            provider_key.fail_count += 1
            await self._record_usage(
                db, user, model, provider.name, 0, 0, 0,
                duration, False, resp.status_code,
                resp.text[:500], request_id,
            )
            raise ValueError(f"Provider returned {resp.status_code}: {resp.text[:500]}")

        data = resp.json()
        usage = data.get("usage", {})
        prompt_tokens = usage.get("prompt_tokens", 0)
        completion_tokens = usage.get("completion_tokens", 0)
        total_tokens = usage.get("total_tokens", 0)
        cost = calculate_cost(model, prompt_tokens, completion_tokens)

        # Record usage
        await self._record_usage(
            db, user, model, provider.name,
            prompt_tokens, completion_tokens, total_tokens,
            duration, True, 200, None, request_id, cost,
        )

        # Success - reset fail count
        provider_key.fail_count = max(0, provider_key.fail_count - 1)
        provider_key.last_success_at = time.time()
        provider.health_status = "healthy"

        return data

    async def _handle_stream(
        self, client, provider, provider_key, db, user,
        model, request_id, base_url, headers, body, start_time,
    ) -> AsyncGenerator[bytes, None]:
        """Handle streaming (SSE) response."""
        prompt_tokens = 0
        completion_tokens = 0

        async with client.stream(
            "POST",
            f"{base_url}/v1/chat/completions",
            headers=headers,
            json=body,
        ) as resp:
            duration = int((time.time() - start_time) * 1000)

            if resp.status_code != 200:
                provider_key.fail_count += 1
                error_text = await resp.aread()
                await self._record_usage(
                    db, user, model, provider.name, 0, 0, 0,
                    duration, True, resp.status_code,
                    error_text.decode()[:500], request_id,
                )
                yield json.dumps({
                    "error": f"Provider returned {resp.status_code}",
                }).encode()
                return

            async for line in resp.aiter_lines():
                if line.startswith("data: "):
                    chunk = line[6:]
                    if chunk == "[DONE]":
                        yield b"data: [DONE]\n\n"
                        break
                    yield f"data: {chunk}\n\n".encode()

                    # Count tokens from streaming response
                    try:
                        chunk_data = json.loads(chunk)
                        usage = chunk_data.get("usage", {})
                        if usage:
                            prompt_tokens = usage.get("prompt_tokens", prompt_tokens)
                            completion_tokens = usage.get(
                                "completion_tokens", completion_tokens
                            )
                    except json.JSONDecodeError:
                        pass

            total = prompt_tokens + completion_tokens
            cost = calculate_cost(model, prompt_tokens, completion_tokens)
            await self._record_usage(
                db, user, model, provider.name,
                prompt_tokens or 0, completion_tokens or 0, total or 0,
                duration, True, 200, None, request_id, cost,
            )

            provider_key.fail_count = max(0, provider_key.fail_count - 1)
            provider_key.last_success_at = time.time()

    async def _record_usage(
        self, db, user, model, provider_name,
        prompt_tokens, completion_tokens, total_tokens,
        duration_ms, is_success, status_code, error_message,
        request_id, cost=None,
    ):
        """Record usage log."""
        if cost is None:
            cost = calculate_cost(model, prompt_tokens, completion_tokens)

        log = UsageLog(
            user_id=user.id,
            model=model,
            provider=provider_name,
            prompt_tokens=prompt_tokens,
            completion_tokens=completion_tokens,
            total_tokens=total_tokens,
            cost_rmb=cost,
            duration_ms=duration_ms,
            is_success=is_success,
            status_code=status_code,
            error_message=error_message,
            request_id=request_id,
        )
        db.add(log)

        # Update user quota
        if is_success:
            user.quota_used += cost
            user.quota_balance -= cost

        await db.flush()

    async def list_models(
        self, db: AsyncSession, user: Optional[User] = None
    ) -> list[str]:
        """List available models.
        
        For admin/super_admin: returns all real models from providers.
        For regular users: returns their allowed model aliases.
        """
        if user and user.role not in ("admin", "super_admin", "finance"):
            # Return user's allowed aliases
            allowed = user.allowed_models or []
            if not allowed:
                return []
            # Resolve aliases to check which are active
            result = await db.execute(
                select(ModelAlias).where(
                    ModelAlias.alias_name.in_(allowed),
                    ModelAlias.is_active == True,  # noqa: E712
                )
            )
            aliases = result.scalars().all()
            # Return alias names that are active
            active_aliases = {a.alias_name for a in aliases}
            return [m for m in allowed if m in active_aliases]

        # Admin: return all real models from providers
        result = await db.execute(
            select(Provider).where(Provider.is_active == True)  # noqa: E712
        )
        providers = result.scalars().all()
        models: list[str] = []
        seen: set[str] = set()
        for p in providers:
            for m in (p.models or []):
                if m not in seen:
                    models.append(m)
                    seen.add(m)
        return models


gateway_service = GatewayService()
