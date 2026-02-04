"""
B-FAST: Binary Fast Adaptive Serialization Transfer

Ultra-fast binary serialization library with Rust backend.
"""

from .b_fast import BFast
from .integration import BFastResponse

__version__ = "1.0.3"
__all__ = ["BFast", "BFastResponse"]
