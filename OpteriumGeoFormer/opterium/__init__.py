"""
Opterium — Architecture Flip Package

Encoder → Navigation → Decoder

The encoder and decoder are small trainable layers.
All reasoning happens in the navigation core (zero weights).
"""

from .encoder import TokenEncoder
from .decoder import TokenDecoder
from .navigation import NavigationCore
from .pipeline import OpteriumPipeline

__all__ = ['TokenEncoder', 'TokenDecoder', 'NavigationCore', 'OpteriumPipeline']
