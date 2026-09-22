"""CodSpeed benchmarks for the B-FAST length-prefixed streaming protocol."""

import pytest

from b_fast.streaming import BFastStreamDecoder, BFastStreamEncoder


@pytest.fixture(scope="session")
def stream_encoder():
    return BFastStreamEncoder()


@pytest.fixture(scope="session")
def small_object():
    return {"status": "ok", "items": [1, 2, 3, 4, 5]}


@pytest.fixture(scope="session")
def single_frame(stream_encoder, small_object):
    return stream_encoder.encode_frame(small_object, compress=False)


@pytest.fixture(scope="session")
def frames_1k(stream_encoder, users_1k):
    encoder = BFastStreamEncoder()
    return b"".join(encoder.encode_frame(user, compress=False) for user in users_1k)


@pytest.fixture(scope="session")
def fragmented_frames(frames_1k):
    """Same payload as frames_1k, split on arbitrary (non frame-aligned) boundaries."""
    chunk_size = 997
    return [frames_1k[i : i + chunk_size] for i in range(0, len(frames_1k), chunk_size)]


# --- Encoding ------------------------------------------------------------------


def test_stream_encode_frame_small(benchmark, stream_encoder, small_object):
    benchmark(lambda: stream_encoder.encode_frame(small_object, compress=False))


def test_stream_encode_frame_compressed(benchmark, stream_encoder, users_100):
    benchmark(lambda: stream_encoder.encode_frame(users_100, compress=True))


def test_stream_encode_1k_frames(benchmark, stream_encoder, users_1k):
    def encode_all():
        return [stream_encoder.encode_frame(user, compress=False) for user in users_1k]

    benchmark(encode_all)


# --- Decoding ------------------------------------------------------------------


def test_stream_decode_single_frame(benchmark, single_frame):
    def decode():
        decoder = BFastStreamDecoder(expect_handshake=False)
        return decoder.feed(single_frame)

    benchmark(decode)


def test_stream_decode_1k_frames(benchmark, frames_1k):
    def decode():
        decoder = BFastStreamDecoder(expect_handshake=False)
        return decoder.feed(frames_1k)

    benchmark(decode)


def test_stream_decode_fragmented(benchmark, fragmented_frames):
    """Worst case for the sliding buffer: frames split across network chunks."""

    def decode():
        decoder = BFastStreamDecoder(expect_handshake=False)
        decoded = 0
        for chunk in fragmented_frames:
            decoded += len(decoder.feed(chunk))
        return decoded

    benchmark(decode)
