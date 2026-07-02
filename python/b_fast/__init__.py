"""
B-FAST: Binary Fast Adaptive Serialization Transfer

Ultra-fast binary serialization library with Rust backend.
"""

from ._b_fast import BFast, BFastError
from .integration import BFastResponse

__version__ = "1.3.0"
__all__ = ["BFast", "BFastError", "BFastResponse"]
