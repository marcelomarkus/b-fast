"""
B-FAST: Binary Fast Adaptive Serialization Transfer

Ultra-fast binary serialization library with Rust backend.
"""

from ._b_fast import BFast, BFastError
from .integration import BFastResponse

__version__ = "1.2.1"
__all__ = ["BFast", "BFastError", "BFastResponse"]
