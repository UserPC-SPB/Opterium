# geo_matmul_v2.pyx  —  Correct optimized geometric matrix multiply
#
# C[i][j] = Σ_k ((S_ik²-D_ik²)//4) · ((S_kj²-D_kj²)//4)
# Pure int. Zero float. Optimized: flat arrays, cache-friendly i→k→j loop.

from libc.stdlib cimport malloc, calloc, free

def sd_matmul_v2(list A_sd, list B_sd):
    """Optimized geometric matrix multiply.

    A_sd: list of lists of (int, int) — (S, D) for each element
    B_sd: list of lists of (int, int) — (S, D) for each element
    Returns: list of lists of int — matrix product values
    """
    cdef int m = len(A_sd)
    cdef int k = len(A_sd[0]) if m > 0 else 0
    cdef int n = len(B_sd[0]) if B_sd else 0

    if k != len(B_sd):
        raise ValueError(f"Shape mismatch: ({m}x{k}) x ({len(B_sd)}x{n})")

    # Flatten A: S_a[i*k + p], D_a[i*k + p]
    cdef int* S_a = <int*>malloc(m * k * sizeof(int))
    cdef int* D_a = <int*>malloc(m * k * sizeof(int))
    if not S_a or not D_a:
        free(S_a); free(D_a); raise MemoryError("A alloc failed")

    cdef int i, j, p
    cdef list row
    cdef tuple pair

    for i in range(m):
        row = A_sd[i]
        for p in range(k):
            pair = row[p]
            S_a[i * k + p] = pair[0]
            D_a[i * k + p] = pair[1]

    # Flatten B: S_b[p*n + j], D_b[p*n + j]
    cdef int* S_b = <int*>malloc(k * n * sizeof(int))
    cdef int* D_b = <int*>malloc(k * n * sizeof(int))
    if not S_b or not D_b:
        free(S_a); free(D_a); free(S_b); free(D_b)
        raise MemoryError("B alloc failed")

    for p in range(k):
        row = B_sd[p]
        for j in range(n):
            pair = row[j]
            S_b[p * n + j] = pair[0]
            D_b[p * n + j] = pair[1]

    # Result buffer
    cdef int* C = <int*>calloc(m * n, sizeof(int))
    if not C:
        free(S_a); free(D_a); free(S_b); free(D_b)
        raise MemoryError("C alloc failed")

    # Core i→k→j multiply-accumulate (cache-friendly: j inner loop)
    cdef int S1, D1, P1, S2, D2, P2
    for i in range(m):
        for p in range(k):
            S1 = S_a[i * k + p]
            D1 = D_a[i * k + p]
            P1 = (S1 * S1 - D1 * D1) // 4
            for j in range(n):
                S2 = S_b[p * n + j]
                D2 = D_b[p * n + j]
                P2 = (S2 * S2 - D2 * D2) // 4
                C[i * n + j] += P1 * P2

    # Convert back
    result = [[0] * n for _ in range(m)]
    for i in range(m):
        for j in range(n):
            result[i][j] = C[i * n + j]

    free(S_a); free(D_a); free(S_b); free(D_b); free(C)
    return result
