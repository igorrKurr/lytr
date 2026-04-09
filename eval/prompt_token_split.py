"""
Approximate OpenAI chat **content** token counts (system vs user strings) using tiktoken.

API `usage.prompt_tokens` includes chat framing; these fields count only the message bodies
so you can compare fixed system overhead vs task-specific user text. Requires `tiktoken`
for accuracy (`pip install tiktoken`); otherwise uses a rough len/4 fallback.
"""
from __future__ import annotations


def _encoding_for_chat_model(model: str):
    try:
        import tiktoken
    except ImportError:
        return None
    try:
        return tiktoken.encoding_for_model(model)
    except KeyError:
        for name in ("o200k_base", "cl100k_base"):
            try:
                return tiktoken.get_encoding(name)
            except KeyError:
                continue
        return None


def content_tokens(model: str, text: str) -> int:
    """Token count for a single message body (same encoding family as the chat model)."""
    enc = _encoding_for_chat_model(model)
    if enc is not None:
        return len(enc.encode(text))
    return max(1, len(text) // 4)


def split_system_user_tokens(model: str, system: str, user: str) -> tuple[int, int]:
    """Return (tokens_system_content, tokens_user_content)."""
    return content_tokens(model, system), content_tokens(model, user)
