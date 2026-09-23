import json
import shutil
import subprocess
from pathlib import Path

import pytest

from b_fast.streaming import BFastStreamEncoder

CLIENT_TS_DIST = Path(__file__).resolve().parents[1] / "client-ts" / "dist" / "index.js"


def test_cross_language_streaming_roundtrip(tmp_path):
    """End-to-end cross-language test: Python encodes stream -> TypeScript decodes stream."""
    if not shutil.which("node"):
        pytest.skip("Node.js is not available")
    if not CLIENT_TS_DIST.exists():
        pytest.skip(f"TypeScript client not compiled ({CLIENT_TS_DIST} missing)")
    encoder = BFastStreamEncoder()

    items = [
        {"id": 1, "name": "Alice", "tags": ["admin", "dev"], "active": True},
        {"id": 2, "name": "Bob", "tags": ["user"], "active": False},
        {"id": 3, "score": 98.6, "metadata": {"level": 5, "verified": True}},
    ]

    # Generate full stream in Python
    stream_chunks = list(encoder.encode_stream(items, compress=True))
    stream_file = tmp_path / "stream_data.bin"
    output_json_file = tmp_path / "decoded_output.json"

    with open(stream_file, "wb") as f:
        for chunk in stream_chunks:
            f.write(chunk)

    # Node.js script that reads the binary stream and writes decoded JSON
    node_script = f"""
    const fs = require('fs');
    const {{ BFastStreamDecoder }} = require('./client-ts/dist/index.js');

    const streamBytes = fs.readFileSync('{stream_file}');
    const decoder = new BFastStreamDecoder();
    const decoded = decoder.feed(new Uint8Array(streamBytes));
    fs.writeFileSync('{output_json_file}', JSON.stringify(decoded));
    """

    res = subprocess.run(
        ["node", "-e", node_script],
        capture_output=True,
        text=True,
    )
    assert res.returncode == 0, f"Node script failed: {res.stderr}"

    # Read back and compare
    with open(output_json_file) as f:
        decoded_from_ts = json.load(f)

    assert decoded_from_ts == items


def test_cross_language_ts_encode_python_decode(tmp_path):
    """End-to-end cross-language test: TypeScript encodes packet -> Python decodes packet."""
    if not shutil.which("node"):
        pytest.skip("Node.js is not available")
    if not CLIENT_TS_DIST.exists():
        pytest.skip(f"TypeScript client not compiled ({CLIENT_TS_DIST} missing)")

    from b_fast import BFast

    items = {
        "id": 42,
        "name": "Marcelo",
        "scores": [10.5, 20.5],
        "active": True,
        "role": "admin",
    }

    packet_file = tmp_path / "ts_packet.bin"

    node_script = f"""
    const fs = require('fs');
    const {{ BFastEncoder }} = require('./client-ts/dist/index.js');

    const data = {json.dumps(items)};
    const bytes = BFastEncoder.encode(data, {{ compress: true }});
    fs.writeFileSync('{packet_file}', bytes);
    """

    res = subprocess.run(
        ["node", "-e", node_script],
        capture_output=True,
        text=True,
    )
    assert res.returncode == 0, f"Node script failed: {res.stderr}"

    with open(packet_file, "rb") as f:
        raw_bytes = f.read()

    bf = BFast()
    decoded = bf.decode_packed(raw_bytes, decompress=True)
    assert decoded == items
