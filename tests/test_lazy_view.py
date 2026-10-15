"""Tests for BFastLazyView on-demand zero-allocation deserialization."""

from datetime import date, datetime, time
from decimal import Decimal
from uuid import UUID

import pytest
from pydantic import BaseModel

import b_fast


class Product(BaseModel):
    id: int
    name: str
    price: float
    active: bool


def test_lazy_view_dict_primitives():
    encoder = b_fast.BFast()
    data = {
        "id": 101,
        "name": "Super Fast Wire",
        "price": 199.99,
        "active": True,
        "payload": b"binary data",
        "notes": None,
    }
    encoded = encoder.encode_packed(data, compress=False)
    view = b_fast.decode_lazy(encoded, decompress=False)

    assert view.is_dict()
    assert not view.is_list()
    assert len(view) == 6
    assert view["id"] == 101
    assert view["name"] == "Super Fast Wire"
    assert view["price"] == 199.99
    assert view["active"] is True
    assert view["payload"] == b"binary data"
    assert view["notes"] is None


def test_lazy_view_dict_helpers():
    encoder = b_fast.BFast()
    data = {"alpha": 1, "beta": 2, "gamma": 3}
    encoded = encoder.encode_packed(data, compress=False)
    view = b_fast.decode_lazy(encoded, decompress=False)

    assert "alpha" in view
    assert "delta" not in view
    assert view.get("alpha") == 1
    assert view.get("delta") is None
    assert view.get("delta", 999) == 999
    assert set(view.keys()) == {"alpha", "beta", "gamma"}

    with pytest.raises(KeyError):
        _ = view["delta"]


def test_lazy_view_list_operations():
    encoder = b_fast.BFast()
    items = ["first", "second", "third", "fourth", "fifth"]
    encoded = encoder.encode_packed(items, compress=False)
    view = b_fast.decode_lazy(encoded, decompress=False)

    assert view.is_list()
    assert not view.is_dict()
    assert len(view) == 5
    assert view[0] == "first"
    assert view[2] == "third"
    assert view[-1] == "fifth"
    assert view[-2] == "fourth"

    with pytest.raises(IndexError):
        _ = view[10]

    with pytest.raises(IndexError):
        _ = view[-10]

    assert view.to_list() == items


def test_lazy_view_nested_structures():
    encoder = b_fast.BFast()
    data = {
        "group": "Engineering",
        "metadata": {"floor": 3, "city": "SP"},
        "tags": ["prod", "api", "core"],
        "records": [
            {"id": 1, "val": 10.5},
            {"id": 2, "val": 20.5},
        ],
    }
    encoded = encoder.encode_packed(data, compress=False)
    view = b_fast.decode_lazy(encoded, decompress=False)

    assert view["group"] == "Engineering"
    assert view["metadata"]["city"] == "SP"
    assert view["tags"][1] == "api"
    assert view["records"][1]["id"] == 2
    assert view["records"][1]["val"] == 20.5

    # Full materialization
    assert view.materialize() == data


def test_lazy_view_compressed():
    encoder = b_fast.BFast()
    products = [
        Product(id=i, name=f"Product {i}", price=i * 1.5, active=(i % 2 == 0))
        for i in range(100)
    ]
    encoded = encoder.encode_packed(products, compress=True)
    view = b_fast.decode_lazy(encoded, decompress=True)

    assert len(view) == 100
    assert view[50]["id"] == 50
    assert view[50]["name"] == "Product 50"
    assert view[50]["price"] == 75.0
    assert view[50]["active"] is True


def test_lazy_view_extended_types():
    encoder = b_fast.BFast()
    dt = datetime(2026, 10, 15, 10, 30, 0)
    d = date(2026, 10, 15)
    t = time(10, 30, 0)
    uid = UUID("9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d")
    dec = Decimal("99.95")

    data = {
        "timestamp": dt,
        "day": d,
        "clock": t,
        "user_uuid": uid,
        "amount": dec,
    }
    encoded = encoder.encode_packed(data, compress=False)
    view = b_fast.decode_lazy(encoded, decompress=False)

    assert view["timestamp"] == dt
    assert view["day"] == d
    assert view["clock"] == t
    assert view["user_uuid"] == uid
    assert view["amount"] == dec
