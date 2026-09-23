"""
Data science extensions for B-FAST: Polars, Pandas, and PyArrow integration.

Provides ultra-fast serialization and deserialization for DataFrames,
Series, and Tables across orientations (records, columns, split).
"""

from typing import Any, Literal, Union

from ._b_fast import BFast


def encode_dataframe(
    df: Any,
    orient: Literal["records", "columns", "split"] = "records",
    compress: bool = True,
) -> bytes:
    """
    Encode a DataFrame, LazyFrame, Series, or Arrow Table to B-FAST binary format.

    Args:
        df: Polars DataFrame/LazyFrame/Series, Pandas DataFrame/Series, or PyArrow Table/RecordBatch.
        orient: Layout format:
            - 'records': List of row dicts [ {col: val, ...}, ... ]. Best for REST APIs and frontends.
            - 'columns': Columnar dict { col: [values, ...] }. Blazing fast, minimal memory.
            - 'split': Dict with {'columns': [...], 'data': [[...], ...]}.
        compress: Whether to enable LZ4 compression (default True).

    Returns:
        B-FAST binary bytes.
    """
    encoder = BFast()
    type_name = type(df).__name__

    # Polars LazyFrame -> collect first
    if type_name == "LazyFrame" and hasattr(df, "collect"):
        df = df.collect()
        type_name = type(df).__name__

    if orient == "records":
        # Handled directly by Rust fast path
        return encoder.encode_packed(df, compress=compress)

    elif orient == "columns":
        if hasattr(df, "to_dict"):
            try:
                # Polars: df.to_dict(as_series=False)
                data = df.to_dict(as_series=False)
            except TypeError:
                # Pandas: df.to_dict(orient="list")
                data = df.to_dict(orient="list")
        elif hasattr(df, "column_names"):
            # PyArrow Table
            data = {col: df[col].to_pylist() for col in df.column_names}
        elif hasattr(df, "to_list"):
            # Series
            name = getattr(df, "name", "value") or "value"
            data = {str(name): df.to_list()}
        else:
            raise ValueError(
                f"Object of type '{type_name}' cannot be serialized to columnar orientation."
            )
        return encoder.encode_packed(data, compress=compress)

    elif orient == "split":
        if hasattr(df, "to_dict"):
            try:
                # Pandas: df.to_dict(orient="split")
                split_dict = df.to_dict(orient="split")
                data = {
                    "columns": list(split_dict["columns"]),
                    "data": split_dict["data"],
                }
            except TypeError:
                # Polars
                data = {
                    "columns": list(df.columns),
                    "data": [list(row) for row in df.rows()],
                }
        elif hasattr(df, "column_names"):
            # PyArrow Table
            rows = [list(d.values()) for d in df.to_pylist()]
            data = {"columns": list(df.column_names), "data": rows}
        else:
            raise ValueError(
                f"Object of type '{type_name}' cannot be serialized to split orientation."
            )
        return encoder.encode_packed(data, compress=compress)

    else:
        raise ValueError(
            f"Unsupported orient: '{orient}'. Use 'records', 'columns', or 'split'."
        )


def decode_dataframe(
    data: Union[bytes, bytearray, memoryview],
    engine: Literal["auto", "polars", "pandas", "arrow"] = "auto",
) -> Any:
    """
    Decode B-FAST binary data into a DataFrame or Table.

    Supports payloads serialized in 'records', 'columns', or 'split' format.

    Args:
        data: B-FAST binary bytes.
        engine: DataFrame engine to instantiate:
            - 'auto': Prefers Polars if installed, falls back to Pandas, else returns raw Python data.
            - 'polars': Returns a polars.DataFrame.
            - 'pandas': Returns a pandas.DataFrame.
            - 'arrow': Returns a pyarrow.Table.

    Returns:
        DataFrame or Table in requested engine format.
    """
    encoder = BFast()
    raw = encoder.decode_packed(data)

    target_engine = engine
    if target_engine == "auto":
        try:
            import polars  # noqa: F401

            target_engine = "polars"
        except ImportError:
            try:
                import pandas  # noqa: F401

                target_engine = "pandas"
            except ImportError:
                return raw

    if target_engine == "polars":
        import polars as pl

        if isinstance(raw, dict) and "columns" in raw and "data" in raw:
            return pl.DataFrame(raw["data"], schema=raw["columns"], orient="row")
        return pl.DataFrame(raw)

    elif target_engine == "pandas":
        import pandas as pd

        if isinstance(raw, dict) and "columns" in raw and "data" in raw:
            return pd.DataFrame(data=raw["data"], columns=raw["columns"])
        return pd.DataFrame(raw)

    elif target_engine == "arrow":
        import pyarrow as pa

        if isinstance(raw, list):
            return pa.Table.from_pylist(raw)
        elif isinstance(raw, dict):
            if "columns" in raw and "data" in raw:
                import pandas as pd

                df = pd.DataFrame(data=raw["data"], columns=raw["columns"])
                return pa.Table.from_pandas(df)
            else:
                return pa.Table.from_pydict(raw)

    raise ValueError(
        f"Unsupported engine: '{target_engine}'. Use 'auto', 'polars', 'pandas', or 'arrow'."
    )
