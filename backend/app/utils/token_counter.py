"""Token counting utilities (for estimation when provider doesn't return usage)."""

import math
import re


def estimate_tokens(text: str) -> int:
    """Estimate token count for a text string.

    Rough estimation: ~1 token per 4 chars for English, ~1 token per 1.5 chars for CJK.
    """
    if not text:
        return 0

    # Count CJK characters
    cjk_chars = len(re.findall(r"[\u4e00-\u9fff\u3400-\u4dbf\uf900-\ufaff]", text))
    # Count other characters
    other_chars = len(text) - cjk_chars

    # CJK: ~1 token per 1.5 chars, English: ~1 token per 4 chars
    estimated = math.ceil(cjk_chars / 1.5 + other_chars / 4)
    return max(1, estimated)


def estimate_messages_tokens(messages: list[dict]) -> int:
    """Estimate total tokens in a message list."""
    total = 0
    for msg in messages:
        total += 4  # Base overhead per message
        for key, value in msg.items():
            if isinstance(value, str):
                total += estimate_tokens(value)
            elif isinstance(value, list):
                for item in value:
                    if isinstance(item, dict):
                        for v in item.values():
                            if isinstance(v, str):
                                total += estimate_tokens(v)
    total += 2  # Base reply overhead
    return total
