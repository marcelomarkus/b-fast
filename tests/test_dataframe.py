"""
Tests for B-FAST DataFrame integration with Polars, Pandas, and PyArrow.
"""

import pandas as pd
import polars as pl
import pytest

from b_fast import BFast, decode_dataframe, encode_dataframe


@pytest.fixture
def sample_records():
    return [
        {"id": 1, "name": "Alice", "score": 95.5, "active": True},
        {"id": 2, "name": "Bob", "score": 88.0, "active": False},
        {"id": 3, "name": "Charlie", "score": 72.3, "active": True},
    ]


@pytest.fixture
def polars_df(sample_records):
    return pl.DataFrame(sample_records)


@pytest.fixture
def pandas_df(sample_records):
    return pd.DataFrame(sample_records)


def test_native_polars_dataframe_serialization(polars_df, sample_records):
    bf = BFast()
    packed = bf.encode_packed(polars_df, compress=True)
    decoded = bf.decode_packed(packed)
    assert decoded == sample_records


def test_native_pandas_dataframe_serialization(pandas_df, sample_records):
    bf = BFast()
    packed = bf.encode_packed(pandas_df, compress=True)
    decoded = bf.decode_packed(packed)
    assert decoded == sample_records


def test_polars_series_serialization():
    bf = BFast()
    series = pl.Series("temperatures", [21.5, 22.0, 19.8, 25.1])
    packed = bf.encode_packed(series, compress=True)
    decoded = bf.decode_packed(packed)
    assert decoded == [21.5, 22.0, 19.8, 25.1]


def test_pandas_series_serialization():
    bf = BFast()
    series = pd.Series([10, 20, 30], name="counts")
    packed = bf.encode_packed(series, compress=True)
    decoded = bf.decode_packed(packed)
    assert decoded == [10, 20, 30]


def test_polars_lazyframe_serialization(polars_df, sample_records):
    bf = BFast()
    lazy_df = polars_df.lazy().filter(pl.col("score") > 80.0)
    packed = bf.encode_packed(lazy_df, compress=True)
    decoded = bf.decode_packed(packed)
    assert len(decoded) == 2
    assert decoded[0]["name"] == "Alice"
    assert decoded[1]["name"] == "Bob"


def test_nested_dataframe_in_dict(polars_df, sample_records):
    bf = BFast()
    payload = {
        "status": "ok",
        "total": len(polars_df),
        "data": polars_df,
    }
    packed = bf.encode_packed(payload, compress=True)
    decoded = bf.decode_packed(packed)
    assert decoded["status"] == "ok"
    assert decoded["total"] == 3
    assert decoded["data"] == sample_records


@pytest.mark.parametrize("orient", ["records", "columns", "split"])
def test_encode_decode_dataframe_polars(polars_df, sample_records, orient):
    data = encode_dataframe(polars_df, orient=orient, compress=True)
    result = decode_dataframe(data, engine="polars")
    assert isinstance(result, pl.DataFrame)
    assert result.to_dicts() == sample_records


@pytest.mark.parametrize("orient", ["records", "columns", "split"])
def test_encode_decode_dataframe_pandas(pandas_df, sample_records, orient):
    data = encode_dataframe(pandas_df, orient=orient, compress=True)
    result = decode_dataframe(data, engine="pandas")
    assert isinstance(result, pd.DataFrame)
    assert result.to_dict(orient="records") == sample_records


def test_decode_dataframe_auto_engine(polars_df, sample_records):
    data = encode_dataframe(polars_df, orient="records")
    # auto prefers polars if available
    result = decode_dataframe(data, engine="auto")
    assert isinstance(result, pl.DataFrame)
    assert result.to_dicts() == sample_records


def test_encode_dataframe_invalid_orient(polars_df):
    with pytest.raises(ValueError, match="Unsupported orient"):
        encode_dataframe(polars_df, orient="invalid_orient")  # type: ignore
