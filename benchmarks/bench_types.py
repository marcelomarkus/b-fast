"""CodSpeed benchmarks for NumPy payloads and extended Python types."""

import pytest

import b_fast


@pytest.fixture(scope="session")
def encoder():
    return b_fast.BFast()


@pytest.fixture(scope="session")
def packed_numpy(numpy_payload):
    return b_fast.BFast().encode_packed(numpy_payload, compress=False)


@pytest.fixture(scope="session")
def packed_numpy_compressed(numpy_payload):
    return b_fast.BFast().encode_packed(numpy_payload, compress=True)


@pytest.fixture(scope="session")
def packed_extended_1k(extended_users_1k):
    return b_fast.BFast().encode_packed(extended_users_1k, compress=False)


# --- NumPy ---------------------------------------------------------------------


def test_encode_numpy(benchmark, encoder, numpy_payload):
    benchmark(lambda: encoder.encode_packed(numpy_payload, compress=False))


def test_encode_numpy_compressed(benchmark, encoder, numpy_payload):
    benchmark(lambda: encoder.encode_packed(numpy_payload, compress=True))


def test_encode_numpy_large(benchmark, encoder, numpy_large_payload):
    benchmark(lambda: encoder.encode_packed(numpy_large_payload, compress=False))


def test_decode_numpy(benchmark, encoder, packed_numpy):
    benchmark(lambda: encoder.decode_packed(packed_numpy))


def test_decode_numpy_compressed(benchmark, encoder, packed_numpy_compressed):
    benchmark(lambda: encoder.decode_packed(packed_numpy_compressed))


# --- Extended types (datetime, UUID, Decimal, Enum, bytes, tuple, set) ---------


def test_encode_extended_types_1k(benchmark, encoder, extended_users_1k):
    benchmark(lambda: encoder.encode_packed(extended_users_1k, compress=False))


def test_decode_extended_types_1k(benchmark, encoder, packed_extended_1k):
    benchmark(lambda: encoder.decode_packed(packed_extended_1k))
