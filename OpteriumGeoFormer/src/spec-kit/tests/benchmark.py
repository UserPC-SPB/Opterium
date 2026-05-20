"""
benchmark.py  —  Wall-time benchmark for all geometric MM methods.

Compares: torch.matmul, pt_naive, pt_naive_fast, pytable_matmul, sd_matmul
Matrix sizes: [4, 8, 16, 32, 64, 128, 256]
Metrics: wall-time (ms), relative speedup vs torch.matmul
Output: CSV to results/benchmark.csv
"""

import sys, os, time, random, csv
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from methods.pt_naive import pt_naive, pt_naive_fast
from methods.pytable_mm import pytable_matmul, pytable_matmul_cached
from methods.sd_matmul import sd_matmul_from_ints
from methods.baseline import torch_matmul, HAS_TORCH, HAS_NUMPY

RESULTS_DIR = os.path.join(os.path.dirname(__file__), '..', 'results')
os.makedirs(RESULTS_DIR, exist_ok=True)

random.seed(42)

SIZES = [4, 8, 16, 32, 64, 128]
WARMUP = 3
MEASURED = 10

METHODS = []

if HAS_TORCH:
    METHODS.append(("torch.matmul", lambda A, B: torch_matmul(A, B)[0]))
elif HAS_NUMPY:
    METHODS.append(("numpy.dot", lambda A, B: numpy_matmul(A, B)[0]))

METHODS.extend([
    ("pt_naive", lambda A, B: pt_naive(A, B)[0]),
    ("pt_naive_fast", lambda A, B: pt_naive_fast(A, B)[0]),
    ("pytable_matmul", lambda A, B: pytable_matmul(A, B)[0]),
    ("pytable_cached", lambda A, B: pytable_matmul_cached(A, B)[0]),
    ("sd_matmul", lambda A, B: sd_matmul_from_ints(A, B)[0]),
])


def make_matrix(n, max_val=100):
    return [[random.randint(1, max_val) for _ in range(n)] for _ in range(n)]


def time_method(fn, A, B, warmup=WARMUP, measured=MEASURED):
    for _ in range(warmup):
        fn(A, B)

    times = []
    for _ in range(measured):
        t0 = time.perf_counter()
        fn(A, B)
        t1 = time.perf_counter()
        times.append((t1 - t0) * 1000)  # ms

    mean = sum(times) / len(times)
    std = (sum((t - mean) ** 2 for t in times) / len(times)) ** 0.5
    return mean, std


def run_benchmark():
    print("=== Geometric MM Benchmark ===\n")
    print(f"{'Size':>6} {'Method':>20} {'Mean (ms)':>12} {'Std (ms)':>12} {'Speedup':>10}")
    print("-" * 62)

    results = []

    for n in SIZES:
        A = make_matrix(n)
        B = make_matrix(n)

        # Get baseline timing
        baseline_time = None
        for name, fn in METHODS:
            if 'torch' in name or 'numpy' in name:
                try:
                    baseline_time, _ = time_method(fn, A, B)
                except Exception as e:
                    print(f"  {n:6d} {name:>20}  ERROR: {e}")
                break

        for name, fn in METHODS:
            try:
                t_mean, t_std = time_method(fn, A, B)
                speedup = f"{baseline_time / t_mean:.2f}x" if baseline_time else "N/A"
                results.append((n, name, t_mean, t_std, speedup))
                print(f"  {n:6d} {name:>20} {t_mean:10.3f}ms {t_std:10.3f}ms {speedup:>10}")
            except Exception as e:
                print(f"  {n:6d} {name:>20}  ERROR: {e}")

    # Save CSV
    csv_path = os.path.join(RESULTS_DIR, 'benchmark.csv')
    with open(csv_path, 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['size', 'method', 'mean_ms', 'std_ms', 'speedup'])
        for n, name, t_mean, t_std, speedup in results:
            w.writerow([n, name, round(t_mean, 3), round(t_std, 3), speedup])
    print(f"\nResults saved to {csv_path}")

    return results


def plot_ascii(results):
    """Simple ASCII bar chart of results."""
    print("\n=== ASCII Speedup Chart ===")
    sizes = sorted(set(r[0] for r in results))
    methods = sorted(set(r[1] for r in results if 'torch' not in r[1] and 'numpy' not in r[1]))

    for n in sizes:
        print(f"\nn={n}:")
        for m in methods:
            match = [r for r in results if r[0] == n and r[1] == m]
            if not match:
                continue
            speedup_str = match[0][4]
            if speedup_str == "N/A":
                continue
            speedup = float(speedup_str.replace('x', ''))
            bars = int(speedup * 5)
            bar = '█' * min(bars, 40) + '░' * max(0, 40 - bars)
            print(f"  {m:>20}: {bar} {speedup_str}")


if __name__ == '__main__':
    res = run_benchmark()
    plot_ascii(res)
