"""
Comprehensive tests for Apache Arrow C Data Interface integration in B-FAST.

Validates zero-copy streaming serialization from Polars (and any Arrow C Stream
compatible provider) into B-FAST binary format.
"""

from datetime import date, datetime

import pytest

pl = pytest.importorskip("polars")

from b_fast import BFast, decode_dataframe, encode_dataframe


def test_arrow_all_numeric_types():
    df = pl.DataFrame(
        {
            "i8": pl.Series([1, -2, 3], dtype=pl.Int8),
            "i16": pl.Series([100, -200, 300], dtype=pl.Int16),
            "i32": pl.Series([10000, -20000, 30000], dtype=pl.Int32),
            "i64": pl.Series([10000000000, -20000000000, 30000000000], dtype=pl.Int64),
            "u8": pl.Series([10, 20, 30], dtype=pl.UInt8),
            "u16": pl.Series([1000, 2000, 3000], dtype=pl.UInt16),
            "u32": pl.Series([100000, 200000, 300000], dtype=pl.UInt32),
            "u64": pl.Series([10000000000, 20000000000, 30000000000], dtype=pl.UInt64),
            "f32": pl.Series([1.5, -2.5, 3.5], dtype=pl.Float32),
            "f64": pl.Series(
                [1.123456789, -2.123456789, 3.123456789], dtype=pl.Float64
            ),
        }
    )

    bf = BFast()
    raw = bf.encode_packed(df, compress=True)
    decoded = bf.decode_packed(raw)

    expected = df.to_dicts()
    assert len(decoded) == len(expected)
    for dec_row, exp_row in zip(decoded, expected):
        for k in exp_row:
            if "f32" in k or "f64" in k:
                assert pytest.approx(dec_row[k], rel=1e-5) == exp_row[k]
            else:
                assert dec_row[k] == exp_row[k]


def test_arrow_strings_inlined_and_variadic():
    df = pl.DataFrame(
        {
            # <= 12 bytes: inlined directly in ArrowStringView prefix
            "short_str": ["cat", "dog", "elephant"],
            # > 12 bytes: requires variadic buffer offset lookup
            "long_str": [
                "this is a very long string exceeding 12 bytes definitely",
                "another extraordinarily lengthy piece of text for arrow verification",
                "the third long string to ensure variadic buffer indexing works seamlessly",
            ],
        }
    )

    bf = BFast()
    raw = bf.encode_packed(df, compress=True)
    decoded = bf.decode_packed(raw)

    assert decoded == df.to_dicts()


def test_arrow_booleans():
    df = pl.DataFrame(
        {
            "b_mixed": [True, False, True, True, False, False, True],
            "b_all_true": [True] * 7,
            "b_all_false": [False] * 7,
        }
    )

    bf = BFast()
    raw = bf.encode_packed(df, compress=True)
    decoded = bf.decode_packed(raw)

    assert decoded == df.to_dicts()


def test_arrow_null_values():
    df = pl.DataFrame(
        {
            "opt_int": [1, None, 3, None, 5],
            "opt_float": [1.1, None, 3.3, 4.4, None],
            "opt_str": ["hello", None, "world", None, "b-fast"],
            "opt_bool": [True, None, False, None, True],
        }
    )

    bf = BFast()
    raw = bf.encode_packed(df, compress=True)
    decoded = bf.decode_packed(raw)

    assert decoded == df.to_dicts()


def test_arrow_dates_and_datetimes():
    df = pl.DataFrame(
        {
            "d": [date(2026, 1, 1), date(2026, 11, 26), date(2030, 12, 31)],
            "dt": [
                datetime(2026, 1, 1, 10, 30, 0),
                datetime(2026, 11, 26, 23, 45, 12, 123456),
                datetime(2030, 12, 31, 0, 0, 0),
            ],
        }
    )

    bf = BFast()
    raw = bf.encode_packed(df, compress=True)
    decoded = bf.decode_packed(raw)

    assert decoded == df.to_dicts()


def test_arrow_columnar_orientation():
    df = pl.DataFrame(
        {
            "id": [1, 2, 3, 4],
            "val": [10.5, 20.5, 30.5, 40.5],
            "label": ["A", "B", "C", "D"],
        }
    )

    # Encode with orient="columns"
    raw = encode_dataframe(df, orient="columns", compress=True)
    decoded = decode_dataframe(raw, engine="polars")

    assert decoded.to_dicts() == df.to_dicts()


def test_arrow_multiple_batches():
    # Construct a chunked Polars DataFrame (multiple record batches in Arrow stream)
    df1 = pl.DataFrame({"id": [1, 2, 3], "name": ["a", "b", "c"]})
    df2 = pl.DataFrame({"id": [4, 5, 6], "name": ["d", "e", "f"]})
    chunked_df = pl.concat([df1, df2], rechunk=False)

    bf = BFast()
    raw = bf.encode_packed(chunked_df, compress=True)
    decoded = bf.decode_packed(raw)

    assert decoded == chunked_df.to_dicts()
    assert len(decoded) == 6


def test_arrow_series_zero_copy():
    s = pl.Series("numbers", [10, 20, 30, 40, 50])
    bf = BFast()
    raw = bf.encode_packed(s, compress=True)
    decoded = bf.decode_packed(raw)

    assert decoded == [10, 20, 30, 40, 50]


def test_arrow_large_scale_throughput():
    # 50,000 rows with mixed types
    n = 50_000
    df = pl.DataFrame(
        {
            "id": list(range(n)),
            "score": [float(i) * 0.5 for i in range(n)],
            "city": ["Sao Paulo", "New York", "London", "Tokyo", "Berlin"] * (n // 5),
            "active": [True, False] * (n // 2),
        }
    )

    bf = BFast()
    raw = bf.encode_packed(df, compress=True)
    assert len(raw) > 0

    decoded = bf.decode_packed(raw)
    assert len(decoded) == n
    assert decoded[0] == {"active": True, "city": "Sao Paulo", "id": 0, "score": 0.0}
    assert decoded[-1] == {
        "active": False,
        "city": "Berlin",
        "id": n - 1,
        "score": float(n - 1) * 0.5,
    }
