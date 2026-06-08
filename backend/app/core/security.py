import hashlib
import secrets
import uuid
from datetime import datetime, timedelta, timezone
from typing import Any, Optional

from jose import JWTError, jwt
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from fastapi import HTTPException, status

from app.core.config import get_settings

settings = get_settings()


def create_access_token(
    data: dict[str, Any], expires_delta: Optional[timedelta] = None
) -> str:
    """Create JWT access token with jti claim."""
    to_encode = data.copy()
    expire = datetime.now(timezone.utc) + (
        expires_delta or settings.JWT_ACCESS_TOKEN_EXPIRE_DELTA
    )
    to_encode.update({
        "exp": expire,
        "jti": str(uuid.uuid4()),
        "type": "access",
        "iat": datetime.now(timezone.utc),
    })
    return jwt.encode(to_encode, settings.SECRET_KEY, algorithm=settings.JWT_ALGORITHM)


def create_refresh_token(data: dict[str, Any]) -> str:
    """Create JWT refresh token."""
    to_encode = data.copy()
    expire = datetime.now(timezone.utc) + settings.JWT_REFRESH_TOKEN_EXPIRE_DELTA
    to_encode.update({
        "exp": expire,
        "jti": str(uuid.uuid4()),
        "type": "refresh",
        "iat": datetime.now(timezone.utc),
    })
    return jwt.encode(to_encode, settings.SECRET_KEY, algorithm=settings.JWT_ALGORITHM)


def decode_token(token: str) -> dict[str, Any]:
    """Decode and validate JWT. Raises 401 on failure."""
    try:
        payload = jwt.decode(
            token, settings.SECRET_KEY, algorithms=[settings.JWT_ALGORITHM]
        )
        return payload
    except JWTError as e:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail=f"Invalid token: {str(e)}",
            headers={"WWW-Authenticate": "Bearer"},
        )


def get_token_blacklist_key(jti: str) -> str:
    """Redis key for JWT blacklist entry."""
    return f"jwt_blacklist:{jti}"


def hash_token(token: str) -> str:
    """SHA256 hash of API token."""
    return hashlib.sha256(token.encode()).hexdigest()


def verify_token_hash(token: str, token_hash: str) -> bool:
    """Verify token matches stored hash."""
    return hash_token(token) == token_hash


def generate_api_token() -> str:
    """Generate sk-company-xxxxx format token."""
    return f"sk-company-{secrets.token_hex(20)}"


def encrypt_value(plaintext: str, key: str) -> str:
    """AES-256-GCM encrypt. Returns base64(nonce + ciphertext + tag)."""
    import base64
    key_bytes = key.encode("utf-8")
    # Ensure 32-byte key via SHA256
    aes_key = hashlib.sha256(key_bytes).digest()
    aesgcm = AESGCM(aes_key)
    nonce = secrets.token_bytes(12)
    ciphertext = aesgcm.encrypt(nonce, plaintext.encode("utf-8"), None)
    return base64.b64encode(nonce + ciphertext).decode("utf-8")


def decrypt_value(ciphertext_b64: str, key: str) -> str:
    """AES-256-GCM decrypt."""
    import base64
    key_bytes = key.encode("utf-8")
    aes_key = hashlib.sha256(key_bytes).digest()
    aesgcm = AESGCM(aes_key)
    data = base64.b64decode(ciphertext_b64)
    nonce = data[:12]
    ciphertext = data[12:]
    plaintext = aesgcm.decrypt(nonce, ciphertext, None)
    return plaintext.decode("utf-8")


def mask_sensitive(value: str, show_prefix: int = 3, show_suffix: int = 4) -> str:
    """Mask middle of string: show first N and last M chars."""
    if len(value) <= show_prefix + show_suffix + 4:
        return value
    return value[:show_prefix] + "*" * (len(value) - show_prefix - show_suffix) + value[-show_suffix:]
