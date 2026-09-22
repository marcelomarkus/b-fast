"""Shared payloads for the CodSpeed benchmark suite."""

from datetime import date, datetime, time
from decimal import Decimal
from enum import Enum
from uuid import UUID

import numpy as np
import pytest
from pydantic import BaseModel


class Status(Enum):
    ACTIVE = "active"
    INACTIVE = "inactive"
    PENDING = "pending"


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]
    description: str


class ExtendedUser(BaseModel):
    id: int
    name: str
    created_at: datetime
    birth_date: date
    wake_time: time
    user_id: UUID
    balance: Decimal
    status: Status
    tags: tuple[str, ...]
    unique_ids: set[int]
    payload: bytes


def make_users(count: int) -> list[User]:
    return [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(5)],
            description=f"Description for user {i}",
        )
        for i in range(count)
    ]


def make_extended_users(count: int) -> list[ExtendedUser]:
    return [
        ExtendedUser(
            id=i,
            name=f"User {i}",
            created_at=datetime(2024, 1, 15, 10, 30, 45),
            birth_date=date(1994, 5, 20),
            wake_time=time(7, 30, 0),
            user_id=UUID(int=i),
            balance=Decimal("1234.56"),
            status=Status.ACTIVE,
            tags=("python", "rust", "fastapi"),
            unique_ids={i, i + 1, i + 2},
            payload=b"binary data here",
        )
        for i in range(count)
    ]


def make_nested_documents(count: int) -> list[dict]:
    return [
        {
            "id": i,
            "meta": {
                "source": "sensor",
                "tags": ["alpha", "beta", "gamma"],
                "nested": {"level": 3, "enabled": i % 2 == 0},
            },
            "measures": [
                {"name": f"metric_{j}", "value": float(i * j)} for j in range(5)
            ],
        }
        for i in range(count)
    ]


@pytest.fixture(scope="session")
def users_100() -> list[User]:
    return make_users(100)


@pytest.fixture(scope="session")
def users_1k() -> list[User]:
    return make_users(1000)


@pytest.fixture(scope="session")
def users_10k() -> list[User]:
    return make_users(10000)


@pytest.fixture(scope="session")
def extended_users_1k() -> list[ExtendedUser]:
    return make_extended_users(1000)


@pytest.fixture(scope="session")
def nested_documents_1k() -> list[dict]:
    return make_nested_documents(1000)


@pytest.fixture(scope="session")
def primitives_10k() -> list[int]:
    return list(range(10000))


@pytest.fixture(scope="session")
def numpy_payload() -> dict:
    rng = np.random.default_rng(42)
    return {
        "matrix_f64": rng.random((100, 100)),
        "vector_i64": np.arange(1000, dtype=np.int64),
    }


@pytest.fixture(scope="session")
def numpy_large_payload() -> dict:
    rng = np.random.default_rng(1234)
    return {"matrix_f64": rng.random((500, 500))}
