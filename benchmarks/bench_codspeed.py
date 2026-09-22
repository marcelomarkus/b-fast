"""CodSpeed continuous benchmarking suite for B-FAST: core encode/decode paths."""

import pytest

import b_fast


@pytest.fixture(scope="session")
def encoder():
    return b_fast.BFast()


@pytest.fixture(scope="session")
def packed_100(users_100):
    return b_fast.BFast().encode_packed(users_100, compress=False)


@pytest.fixture(scope="session")
def packed_1k(users_1k):
    return b_fast.BFast().encode_packed(users_1k, compress=False)


@pytest.fixture(scope="session")
def packed_10k(users_10k):
    return b_fast.BFast().encode_packed(users_10k, compress=False)


@pytest.fixture(scope="session")
def packed_compressed_1k(users_1k):
    return b_fast.BFast().encode_packed(users_1k, compress=True)


@pytest.fixture(scope="session")
def packed_compressed_10k(users_10k):
    return b_fast.BFast().encode_packed(users_10k, compress=True)


@pytest.fixture(scope="session")
def packed_primitives_10k(primitives_10k):
    return b_fast.BFast().encode_packed(primitives_10k, compress=False)


@pytest.fixture(scope="session")
def packed_nested_1k(nested_documents_1k):
    return b_fast.BFast().encode_packed(nested_documents_1k, compress=False)


# --- Encoding: Pydantic models -------------------------------------------------


def test_encode_100_uncompressed(benchmark, encoder, users_100):
    benchmark(lambda: encoder.encode_packed(users_100, compress=False))


def test_encode_1k_uncompressed(benchmark, encoder, users_1k):
    benchmark(lambda: encoder.encode_packed(users_1k, compress=False))


def test_encode_1k_compressed(benchmark, encoder, users_1k):
    benchmark(lambda: encoder.encode_packed(users_1k, compress=True))


def test_encode_10k_uncompressed(benchmark, encoder, users_10k):
    benchmark(lambda: encoder.encode_packed(users_10k, compress=False))


def test_encode_10k_compressed_parallel(benchmark, encoder, users_10k):
    """Above PARALLEL_COMPRESSION_THRESHOLD: exercises the rayon compression path."""
    benchmark(lambda: encoder.encode_packed(users_10k, compress=True))


# --- Encoding: plain Python containers -----------------------------------------


def test_encode_nested_dicts_1k(benchmark, encoder, nested_documents_1k):
    benchmark(lambda: encoder.encode_packed(nested_documents_1k, compress=False))


def test_encode_primitives_10k(benchmark, encoder, primitives_10k):
    benchmark(lambda: encoder.encode_packed(primitives_10k, compress=False))


# --- Decoding ------------------------------------------------------------------


def test_decode_100_uncompressed(benchmark, encoder, packed_100):
    benchmark(lambda: encoder.decode_packed(packed_100))


def test_decode_1k_uncompressed(benchmark, encoder, packed_1k):
    benchmark(lambda: encoder.decode_packed(packed_1k))


def test_decode_1k_compressed(benchmark, encoder, packed_compressed_1k):
    benchmark(lambda: encoder.decode_packed(packed_compressed_1k))


def test_decode_10k_uncompressed(benchmark, encoder, packed_10k):
    benchmark(lambda: encoder.decode_packed(packed_10k))


def test_decode_10k_compressed(benchmark, encoder, packed_compressed_10k):
    benchmark(lambda: encoder.decode_packed(packed_compressed_10k))


def test_decode_nested_dicts_1k(benchmark, encoder, packed_nested_1k):
    benchmark(lambda: encoder.decode_packed(packed_nested_1k))


def test_decode_primitives_10k(benchmark, encoder, packed_primitives_10k):
    benchmark(lambda: encoder.decode_packed(packed_primitives_10k))


# --- Round trip ----------------------------------------------------------------


def test_round_trip_1k_uncompressed(benchmark, encoder, users_1k):
    def round_trip():
        return encoder.decode_packed(encoder.encode_packed(users_1k, compress=False))

    benchmark(round_trip)


def test_round_trip_1k_compressed(benchmark, encoder, users_1k):
    def round_trip():
        return encoder.decode_packed(encoder.encode_packed(users_1k, compress=True))

    benchmark(round_trip)
