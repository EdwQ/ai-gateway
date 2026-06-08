"""PII masking utilities."""

import re
from typing import Optional


def mask_phone(text: str) -> str:
    """Mask Chinese phone numbers: 138****1234."""
    return re.sub(r"(1[3-9]\d{2})\d{4}(\d{4})", r"\1****\2", text)


def mask_id_card(text: str) -> str:
    """Mask Chinese ID card numbers."""
    def _mask(match):
        s = match.group(0)
        if len(s) == 18:
            return s[:6] + "********" + s[-4:]
        return s
    return re.sub(r"\b\d{17}[\dXx]\b", _mask, text)


def mask_email(text: str) -> str:
    """Mask email addresses."""
    def _mask(match):
        email = match.group(0)
        parts = email.split("@")
        if len(parts) == 2:
            local = parts[0]
            if len(local) <= 2:
                masked_local = local[0] + "***"
            else:
                masked_local = local[0] + "***" + local[-1]
            return f"{masked_local}@{parts[1]}"
        return email
    return re.sub(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", _mask, text)


def mask_bank_card(text: str) -> str:
    """Mask bank card numbers: 6222********1234."""
    return re.sub(r"\b(\d{4})\d{8,11}(\d{4})\b", r"\1********\2", text)


def mask_api_key(text: str) -> str:
    """Mask API keys (sk-xxx format)."""
    return re.sub(r"(sk-[a-z0-9]+)[a-z0-9]{16,}([a-z0-9]{4})", r"\1****\2", text, flags=re.I)


def mask_all(text: str) -> str:
    """Apply all masking functions."""
    text = mask_phone(text)
    text = mask_id_card(text)
    text = mask_email(text)
    text = mask_bank_card(text)
    text = mask_api_key(text)
    return text


def mask_prompt(prompt: str, mode: str = "masked") -> tuple[Optional[str], Optional[str]]:
    """Mask prompt based on mode. Returns (content, summary)."""
    content = None
    summary = None

    if mode == "full":
        content = prompt
    elif mode == "masked":
        content = mask_all(prompt)
    elif mode == "summary":
        summary = prompt[:100] + "..." if len(prompt) > 100 else prompt

    return content, summary
