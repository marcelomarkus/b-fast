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
