"""Middleware to sanitize sensitive information from logs."""

import json
import re
from typing import Any

from starlette.middleware.base import BaseHTTPMiddleware
from starlette.requests import Request
from starlette.responses import Response

SENSITIVE_PATTERNS: list[tuple[str, str | re.Pattern, str]] = [
    ("authorization", re.compile(r"Bearer\s+\S+", re.I), "Bearer ***"),
    ("cookie", re.compile(r"(cookie\s*:\s*).+", re.I), r"\1***"),
    ("x-api-key", re.compile(r"\S+"), "***"),
    ("api_key", re.compile(r"\S+"), "***"),
    ("password", re.compile(r"\S+"), "***"),
    ("secret", re.compile(r"\S+"), "***"),
    ("token", re.compile(r"\S+"), "***"),
]

# PII patterns for content masking
PII_PATTERNS: list[tuple[re.Pattern, str]] = [
    (re.compile(r"1[3-9]\d{9}"), r"\1****\2"),  # Phone
    (re.compile(r"\d{18}[\dXx]"), lambda m: m.group(0)[:6] + "********" + m.group(0)[-4:]),  # ID card
    (re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b"), lambda m: m.group(0)[0] + "***@" + m.group(0).split("@")[1]),  # Email
    (re.compile(r"\b\d{16,19}\b"), lambda m: m.group(0)[:4] + "********" + m.group(0)[-4:]),  # Bank card
]


def sanitize_headers(headers: dict[str, str]) -> dict[str, str]:
    """Remove sensitive values from headers."""
    sanitized = dict(headers)
    for key_lower, pattern, replacement in SENSITIVE_PATTERNS:
        for header_key in list(sanitized.keys()):
            if header_key.lower() == key_lower:
                sanitized[header_key] = replacement
    return sanitized


def sanitize_body(body: str) -> str:
    """Mask PII in body content."""
    for pattern, replacement in PII_PATTERNS:
        body = pattern.sub(replacement, body)
    return body


class LogSanitizerMiddleware(BaseHTTPMiddleware):
    """Middleware to sanitize sensitive data before logging."""

    async def dispatch(self, request: Request, call_next):
        # Sanitize headers for logging
        sanitized_headers = sanitize_headers(dict(request.headers))

        response = await call_next(request)

        # Add sanitized info to request state for logging
        request.state.sanitized_headers = sanitized_headers

        return response


def sanitize_log_data(data: dict[str, Any]) -> dict[str, Any]:
    """Sanitize a dictionary for logging purposes."""
    sensitive_keys = {
        "authorization", "cookie", "x-api-key", "api_key", "apikey",
        "password", "secret", "token", "access_token", "refresh_token",
        "secret_key", "encryption_key",
    }
    result = {}
    for key, value in data.items():
        if key.lower() in sensitive_keys:
            result[key] = "***"
        elif isinstance(value, str) and len(value) > 8:
            result[key] = sanitize_body(value)
        elif isinstance(value, dict):
            result[key] = sanitize_log_data(value)
        else:
            result[key] = value
    return result
