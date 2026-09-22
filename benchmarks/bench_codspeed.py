"""CodSpeed Continuous Benchmarking Suite for B-FAST."""

import numpy as np
import pytest
from pydantic import BaseModel

import b_fast
from b_fast.streaming import (
    BFastStreamDecoder,
    BFastStreamEncoder,
)


class User(BaseModel):
    id: int
    name: str
    email: str
    active: bool
    scores: list[float]
    description: str


@pytest.fixture(scope="module")
def users_1k():
    return [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(5)],
            description=f"Description for user {i}",
        )
        for i in range(1000)
    ]


@pytest.fixture(scope="module")
def users_10k():
    return [
        User(
            id=i,
            name=f"User {i}",
            email=f"user{i}@example.com",
            active=i % 2 == 0,
            scores=[float(i * j) for j in range(5)],
            description=f"Description for user {i}",
        )
        for i in range(10000)
    ]


@pytest.fixture(scope="module")
def packed_data_1k(users_1k):
    encoder = b_fast.BFast()
    return encoder.encode_packed(users_1k, compress=False)


@pytest.fixture(scope="module")
def packed_compressed_1k(users_1k):
    encoder = b_fast.BFast()
    return encoder.encode_packed(users_1k, compress=True)


@pytest.fixture(scope="module")
def numpy_payload():
    return {
        "matrix_f64": np.random.rand(100, 100),
        "vector_i64": np.arange(1000, dtype=np.int64),
    }


def test_encode_1k_uncompressed(benchmark, users_1k):
    encoder = b_fast.BFast()
    benchmark(lambda: encoder.encode_packed(users_1k, compress=False))


def test_encode_1k_compressed(benchmark, users_1k):
    encoder = b_fast.BFast()
    benchmark(lambda: encoder.encode_packed(users_1k, compress=True))


def test_encode_10k_compressed_parallel(benchmark, users_10k):
    encoder = b_fast.BFast()
    benchmark(lambda: encoder.encode_packed(users_10k, compress=True))


def test_decode_1k_uncompressed(benchmark, packed_data_1k):
    encoder = b_fast.BFast()
    benchmark(lambda: encoder.decode_packed(packed_data_1k))


def test_decode_1k_compressed(benchmark, packed_compressed_1k):
    encoder = b_fast.BFast()
    benchmark(lambda: encoder.decode_packed(packed_compressed_1k))


def test_encode_numpy(benchmark, numpy_payload):
    encoder = b_fast.BFast()
    benchmark(lambda: encoder.encode_packed(numpy_payload, compress=False))


def test_stream_framing(benchmark):
    stream_encoder = BFastStreamEncoder()
    obj = {"status": "ok", "items": [1, 2, 3, 4, 5]}
    frame = stream_encoder.encode_frame(obj, compress=False)

    def run_stream():
        decoder = BFastStreamDecoder(expect_handshake=False)
        decoder.feed(frame)

    benchmark(run_stream)
