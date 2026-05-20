"""
baseline.py  —  torch.matmul and numpy.dot wrappers for comparison.

Not used in geometric methods. Only for baseline performance measurement.
"""

HAS_TORCH = False
HAS_NUMPY = False

try:
    import torch
    HAS_TORCH = True
except ImportError:
    pass

try:
    import numpy as np
    HAS_NUMPY = True
except ImportError:
    pass


def torch_matmul(A, B):
    """Wrapper for torch.matmul. Returns (result, None)."""
    if not HAS_TORCH:
        raise ImportError("torch not available")
    a = torch.tensor(A, dtype=torch.float32)
    b = torch.tensor(B, dtype=torch.float32)
    return torch.matmul(a, b).tolist(), None


def numpy_matmul(A, B):
    """Wrapper for numpy.dot. Returns (result, None)."""
    if not HAS_NUMPY:
        raise ImportError("numpy not available")
    a = np.array(A, dtype=np.float64)
    b = np.array(B, dtype=np.float64)
    return np.dot(a, b).tolist(), None
