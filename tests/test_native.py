import asyncio
import socket
import threading

import pytest
from pydantic import BaseModel

from b_fast import (
    BFastNativeSender,
    BFastSocketStreamer,
    BFastStreamDecoder,
    is_native_available,
    send_native_socket,
)
from b_fast.integration import BFastStreamingResponse


class UserRecord(BaseModel):
    id: int
    name: str
    active: bool


def test_native_availability():
    avail = is_native_available()
    assert isinstance(avail, bool)
    # On Linux, this should be True
    assert avail is True


def test_send_native_socket_small_and_large():
    s1, s2 = socket.socketpair()

    # 1. Small payload (< 4KB)
    small_data = b"B-FAST Native Fast Socket Transport!"
    sent_small = send_native_socket(s1.fileno(), small_data)
    assert sent_small == len(small_data)
    recvd_small = s2.recv(1024)
    assert recvd_small == small_data

    # 2. Large payload (>= 4KB, e.g. 32KB)
    large_data = b"X" * 32768
    sent_large = send_native_socket(s1.fileno(), large_data)
    assert sent_large == len(large_data)

    recvd_large = b""
    while len(recvd_large) < len(large_data):
        chunk = s2.recv(16384)
        if not chunk:
            break
        recvd_large += chunk
    assert recvd_large == large_data

    s1.close()
    s2.close()


def test_native_sender_lifecycle():
    sender = BFastNativeSender()
    assert sender.is_native_active is True
    assert sender.pending_bytes == 0
    assert sender.in_flight_count == 0

    s1, s2 = socket.socketpair()
    decoder = BFastStreamDecoder()

    # Send handshake
    assert sender.send_handshake(s1.fileno()) == 4

    # Send a small frame
    sent1 = sender.send_frame(s1.fileno(), {"msg": "hello", "count": 1}, compress=True)
    assert sent1 > 0

    # Send a large frame (> 4KB)
    large_payload = {"id": 100, "blob": "A" * 8192}
    sent2 = sender.send_frame(s1.fileno(), large_payload, compress=False)
    assert sent2 > 8192
    assert sender.in_flight_count == 0

    # Send EOS
    assert sender.send_eos(s1.fileno()) == 6
    sender.flush(s1.fileno())
    assert sender.in_flight_count == 0

    # Read and decode on receiving socket
    items = []
    while not decoder.is_eos:
        chunk = s2.recv(16384)
        if not chunk:
            break
        items.extend(decoder.feed(chunk))

    assert len(items) == 2
    assert items[0] == {"msg": "hello", "count": 1}
    assert items[1]["id"] == 100
    assert len(items[1]["blob"]) == 8192

    sender.close()
    assert sender.in_flight_count == 0
    assert sender.is_native_active is False

    s1.close()
    s2.close()


def test_socket_streamer_sync():
    s1, s2 = socket.socketpair()
    decoder = BFastStreamDecoder()

    records = [{"id": i, "name": f"item_{i}", "val": i * 1.5} for i in range(25)]

    with BFastSocketStreamer(s1, compress=True, native=True) as streamer:
        assert streamer.fileno == s1.fileno()
        assert streamer.is_native_active is True
        total_sent = streamer.stream(records)
        assert total_sent > 0

    # Read on s2
    received_items = []
    while not decoder.is_eos:
        chunk = s2.recv(8192)
        if not chunk:
            break
        received_items.extend(decoder.feed(chunk))

    assert len(received_items) == 25
    assert received_items[0] == {"id": 0, "name": "item_0", "val": 0.0}
    assert received_items[24] == {"id": 24, "name": "item_24", "val": 36.0}
    assert decoder.is_eos is True

    s1.close()
    s2.close()


@pytest.mark.anyio
async def test_socket_streamer_async():
    s1, s2 = socket.socketpair()
    s1.setblocking(False)
    s2.setblocking(False)
    decoder = BFastStreamDecoder()

    async def generate_async():
        for i in range(15):
            await asyncio.sleep(0.001)
            yield UserRecord(id=i, name=f"user_{i}", active=(i % 2 == 0))

    async with BFastSocketStreamer(s1, compress=False, native=True) as streamer:
        assert streamer.is_native_active is True
        total_sent = await streamer.stream_async(generate_async())
        assert total_sent > 0

    loop = asyncio.get_running_loop()
    received_items = []
    while not decoder.is_eos:
        chunk = await loop.sock_recv(s2, 8192)
        if not chunk:
            break
        received_items.extend(decoder.feed(chunk))

    assert len(received_items) == 15
    assert received_items[0] == {"id": 0, "name": "user_0", "active": True}
    assert received_items[1] == {"id": 1, "name": "user_1", "active": False}
    assert received_items[14] == {"id": 14, "name": "user_14", "active": True}
    assert decoder.is_eos is True

    s1.close()
    s2.close()


def test_socket_streamer_fallback_mode():
    s1, s2 = socket.socketpair()
    decoder = BFastStreamDecoder()

    # native=False explicitly tests non-native fallback path
    streamer = BFastSocketStreamer(s1, compress=True, native=False)
    assert streamer.is_native_active is False
    assert streamer.pending_bytes == 0

    streamer.stream([{"num": 1}, {"num": 2}])
    streamer.close()

    items = []
    while not decoder.is_eos:
        chunk = s2.recv(4096)
        if not chunk:
            break
        items.extend(decoder.feed(chunk))

    assert items == [{"num": 1}, {"num": 2}]
    s1.close()
    s2.close()


def test_socket_streamer_invalid_socket():
    with pytest.raises(TypeError, match="Expected socket"):
        BFastSocketStreamer("not-a-socket")  # type: ignore

    with pytest.raises(ValueError, match="Invalid socket file descriptor"):
        BFastSocketStreamer(-1)


def test_socket_streamer_concurrent_reader():
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.bind(("127.0.0.1", 0))
    server.listen(1)
    port = server.getsockname()[1]

    client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    client.connect(("127.0.0.1", port))
    conn, _ = server.accept()

    decoder = BFastStreamDecoder()
    results = []

    def reader():
        while not decoder.is_eos:
            chunk = conn.recv(65536)
            if not chunk:
                break
            results.extend(decoder.feed(chunk))

    t = threading.Thread(target=reader)
    t.start()

    with BFastSocketStreamer(client, compress=False, native=True) as streamer:
        payloads = [
            {"batch": i, "data": "T" * (5000 if i % 3 == 0 else 50)} for i in range(30)
        ]
        streamer.stream(payloads)

    t.join()

    assert len(results) == 30
    assert len(results[0]["data"]) == 5000
    assert len(results[1]["data"]) == 50
    assert len(results[3]["data"]) == 5000
    assert decoder.is_eos is True

    client.close()
    conn.close()
    server.close()


def test_bfast_streaming_response_native_flag():
    response = BFastStreamingResponse([{"test": 1}], native=True)
    assert response.native is True
    assert response.status_code == 200
    assert response.media_type == "application/x-bfast-stream"


def test_homogeneous_template_streaming():
    s1, s2 = socket.socketpair()
    decoder = BFastStreamDecoder()
    results = []

    def reader():
        while not decoder.is_eos:
            chunk = s2.recv(65536)
            if not chunk:
                break
            results.extend(decoder.feed(chunk))

    t = threading.Thread(target=reader)
    t.start()

    users = [UserRecord(id=i, name=f"user_{i}", active=i % 2 == 0) for i in range(1000)]
    with BFastSocketStreamer(s1, compress=False, native=True) as streamer:
        streamer.stream(users)

    t.join()
    assert len(results) == 1000
    assert results[0] == {"id": 0, "name": "user_0", "active": True}
    assert results[999] == {"id": 999, "name": "user_999", "active": False}
    s1.close()
    s2.close()


def test_heterogeneous_fallback():
    s1, s2 = socket.socketpair()
    decoder = BFastStreamDecoder()
    results = []

    def reader():
        while not decoder.is_eos:
            chunk = s2.recv(65536)
            if not chunk:
                break
            results.extend(decoder.feed(chunk))

    t = threading.Thread(target=reader)
    t.start()

    mixed_items = [
        {"type": "init", "val": 1},
        UserRecord(id=10, name="alice", active=True),
        [1, 2, 3],
        "plain_string",
        42,
        {"type": "final", "val": 99},
    ]
    with BFastSocketStreamer(s1, compress=False, native=True) as streamer:
        streamer.stream(mixed_items)

    t.join()
    assert len(results) == 6
    assert results[0] == {"type": "init", "val": 1}
    assert results[1] == {"id": 10, "name": "alice", "active": True}
    assert results[2] == [1, 2, 3]
    assert results[3] == "plain_string"
    assert results[4] == 42
    assert results[5] == {"type": "final", "val": 99}
    s1.close()
    s2.close()
