from typing import Any, Optional

from pydantic import BaseModel


class ChatMessage(BaseModel):
    role: str  # system, user, assistant
    content: str


class ChatCompletionRequest(BaseModel):
    model: str
    messages: list[ChatMessage]
    stream: bool = False
    temperature: Optional[float] = None
    max_tokens: Optional[int] = None
    top_p: Optional[float] = None
    frequency_penalty: Optional[float] = None
    presence_penalty: Optional[float] = None
    stop: Optional[list[str]] = None
    user: Optional[str] = None


class ChatUsage(BaseModel):
    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0


class ChatChoice(BaseModel):
    index: int
    message: dict[str, Any]
    finish_reason: Optional[str] = None


class ChatCompletionResponse(BaseModel):
    id: str
    object: str = "chat.completion"
    created: int
    model: str
    choices: list[ChatChoice]
    usage: ChatUsage


class DeltaMessage(BaseModel):
    role: Optional[str] = None
    content: Optional[str] = None


class ChatChoiceStream(BaseModel):
    index: int
    delta: DeltaMessage
    finish_reason: Optional[str] = None


class ChatCompletionChunk(BaseModel):
    id: str
    object: str = "chat.completion.chunk"
    created: int
    model: str
    choices: list[ChatChoiceStream]


class ModelInfo(BaseModel):
    id: str
    object: str = "model"
    created: int
    owned_by: str = "system"


class ModelListResponse(BaseModel):
    object: str = "list"
    data: list[ModelInfo]
