import datetime
import uuid
from decimal import Decimal

import numpy as np
import pytest
from pydantic import BaseModel

from b_fast.streaming import (
    BFastStreamDecoder,
    BFastStreamEncoder,
    is_async_iterable,
)


class SampleItem(BaseModel):
    id: int
    name: str
    active: bool


def test_handshake_and_eos_frame():
    encoder = BFastStreamEncoder()
    handshake = encoder.get_handshake()
    assert handshake == b"BS\x01\x00"

    eos = encoder.get_eos_frame()
    assert len(eos) == 6
    assert eos == b"\x00\x00\x00\x00\x00\x00"


def test_basic_frame_encoding_and_decoding():
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    data = {"message": "hello streaming", "count": 42}
    frame = encoder.encode_frame(data, compress=False)

    assert len(frame) > 6
    # First 4 bytes length LE
    payload_len = int.from_bytes(frame[:4], "little")
    assert payload_len == len(frame) - 6
    assert frame[4] == 0x01  # FRAME_DATA

    items = decoder.feed(encoder.get_handshake() + frame)
    assert len(items) == 1
    assert items[0] == data
    assert decoder.handshake_received is True
    assert decoder.is_eos is False
    assert decoder.pending_bytes == 0


def test_streaming_without_handshake():
    """Decoder should gracefully decode raw frames even if handshake is omitted."""
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    data = {"key": "value"}
    frame = encoder.encode_frame(data, compress=True)

    items = decoder.feed(frame)
    assert len(items) == 1
    assert items[0] == data
    assert decoder.handshake_received is True


def test_explicit_expect_handshake():
    encoder = BFastStreamEncoder()
    data = {"key": "value"}
    frame = encoder.encode_frame(data, compress=True)

    # expect_handshake=True: must raise ValueError if no handshake
    strict_decoder = BFastStreamDecoder(expect_handshake=True)
    with pytest.raises(ValueError, match="Expected stream handshake 'BS' not found"):
        strict_decoder.feed(frame)

    # expect_handshake=False: directly decodes raw frames
    raw_decoder = BFastStreamDecoder(expect_handshake=False)
    assert raw_decoder.handshake_received is True
    items = raw_decoder.feed(frame)
    assert len(items) == 1
    assert items[0] == data


def test_unsupported_stream_version():
    decoder = BFastStreamDecoder()
    bad_handshake = b"BS\x02\x00"  # version 2
    with pytest.raises(ValueError, match="Unsupported stream version: 2"):
        decoder.feed(bad_handshake)


def test_incomplete_handshake_fragmentation():
    """Feed handshake 1 byte at a time."""
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    data = {"test": "frag"}
    stream_bytes = encoder.get_handshake() + encoder.encode_frame(data)

    # Feed first 2 bytes ("BS")
    items = decoder.feed(stream_bytes[:2])
    assert items == []
    assert decoder.handshake_received is False

    # Feed remaining bytes
    items = decoder.feed(stream_bytes[2:])
    assert len(items) == 1
    assert items[0] == data
    assert decoder.handshake_received is True


def test_extreme_byte_by_byte_fragmentation():
    """Simulate worst-case TCP fragmentation: feed 1 byte at a time."""
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    dataset = [{"id": i, "label": f"item_{i}", "val": i * 1.5} for i in range(15)]

    # Generate complete stream
    full_stream = b"".join(encoder.encode_stream(dataset, compress=True))

    collected = []
    for byte in full_stream:
        # Feed 1 byte at a time
        results = decoder.feed(bytes([byte]))
        collected.extend(results)

    assert len(collected) == 15
    assert collected == dataset
    assert decoder.is_eos is True
    assert decoder.pending_bytes == 0


def test_multi_frame_single_chunk():
    """Multiple frames packed in a single network chunk."""
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    items = [{"item": 1}, {"item": 2}, {"item": 3}]
    chunk = (
        encoder.get_handshake()
        + b"".join(encoder.encode_frame(it) for it in items)
        + encoder.get_eos_frame()
    )

    decoded = decoder.feed(chunk)
    assert decoded == items
    assert decoder.is_eos is True


def test_empty_feed_and_reset():
    decoder = BFastStreamDecoder()
    assert decoder.feed(b"") == []
    assert decoder.pending_bytes == 0

    encoder = BFastStreamEncoder()
    decoder.feed(encoder.get_handshake())
    assert decoder.handshake_received is True

    decoder.clear()
    assert decoder.handshake_received is False
    assert decoder.pending_bytes == 0

    decoder.reset()
    assert decoder.is_eos is False


def test_max_frame_size_security_check():
    """A frame declaring length exceeding max_frame_size should raise ValueError."""
    decoder = BFastStreamDecoder(max_frame_size=1024)

    # Craft a malicious header claiming 2048 bytes
    fraudulent_header = (2048).to_bytes(4, "little") + b"\x01\x00"

    with pytest.raises(ValueError, match="exceeds maximum allowed 1024"):
        decoder.feed(fraudulent_header)

    # Buffer should be cleared after error
    assert decoder.pending_bytes == 0


def test_unknown_frame_type():
    decoder = BFastStreamDecoder()
    # 4 bytes len (2), frame type 0x99 (invalid), flags 0, 2 bytes payload
    bad_frame = (2).to_bytes(4, "little") + b"\x99\x00\xaa\xbb"

    with pytest.raises(ValueError, match="Unknown frame type: 0x99"):
        decoder.feed(bad_frame)


def test_amortized_buffer_compaction():
    """Verify that buffer compactions execute properly with large volumes of data."""
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    # Create lots of small frames fed sequentially to test read_pos advancement and compaction
    for i in range(100):
        frame = encoder.encode_frame({"index": i, "payload": "x" * 500})
        decoded = decoder.feed(frame)
        assert len(decoded) == 1
        assert decoded[0]["index"] == i

    assert decoder.pending_bytes == 0


def test_sync_encode_decode_stream_generator():
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    records = [{"step": i, "val": i * 10} for i in range(10)]

    stream_generator = encoder.encode_stream(
        records, compress=True, include_handshake=True, include_eos=True
    )

    decoded_items = list(decoder.decode_stream(stream_generator))
    assert decoded_items == records
    assert decoder.is_eos is True


@pytest.mark.anyio
async def test_async_encode_decode_stream():
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    records = [{"async_id": i, "data": f"async_{i}"} for i in range(8)]

    async def async_data_gen():
        for item in records:
            yield item

    stream_chunks = []
    async for chunk in encoder.encode_async_stream(
        async_data_gen(), compress=True, include_handshake=True, include_eos=True
    ):
        stream_chunks.append(chunk)

    async def async_byte_gen():
        for chunk in stream_chunks:
            yield chunk

    results = []
    async for item in decoder.decode_async_stream(async_byte_gen()):
        results.append(item)

    assert results == records
    assert decoder.is_eos is True


def test_streaming_rich_types():
    """Verify all rich types (NumPy, UUID, Datetime, Decimal, Pydantic) in stream frames."""
    encoder = BFastStreamEncoder()
    decoder = BFastStreamDecoder()

    now = datetime.datetime.now(datetime.timezone.utc)
    today = datetime.date.today()
    cur_time = datetime.time(12, 34, 56)
    test_uuid = uuid.uuid4()
    dec = Decimal("1234.56")
    arr = np.array([1.5, 2.5, 3.5])
    model = SampleItem(id=999, name="pydantic_in_stream", active=True)

    items = [
        {"timestamp": now, "date": today, "time": cur_time},
        {"uuid": test_uuid, "decimal": dec},
        {"numpy": arr},
        {"pydantic": model},
    ]

    stream_bytes = b"".join(encoder.encode_stream(items, compress=True))
    decoded = list(decoder.decode_stream([stream_bytes]))

    assert len(decoded) == 4
    # Datetime / Date checks
    assert isinstance(decoded[0]["timestamp"], datetime.datetime)
    assert decoded[0]["date"] == today
    assert decoded[0]["time"] == cur_time
    # UUID & Decimal
    assert decoded[1]["uuid"] == test_uuid
    assert decoded[1]["decimal"] == dec
    # Numpy array decodes to list of floats (matching BFast decode)
    assert decoded[2]["numpy"] == [1.5, 2.5, 3.5]
    # Pydantic (decoded as dict representation)
    assert decoded[3]["pydantic"] == {
        "id": 999,
        "name": "pydantic_in_stream",
        "active": True,
    }


def test_is_async_iterable_helper():
    async def sample_async_gen():
        yield 1

    def sample_sync_gen():
        yield 1

    assert is_async_iterable(sample_async_gen()) is True
    assert is_async_iterable(sample_sync_gen()) is False
    assert is_async_iterable([1, 2, 3]) is False
