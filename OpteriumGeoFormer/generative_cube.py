"""
GenerativeCube — 3D куб с генеративным развёртыванием узлов.

Не хранит весь куб (1024³ = 1B узлов = 28GB).
Генерирует узел по адресу: O(1).
Bucket index для соседей.
Morpho-связи (живые, усиливаются).

Вдохновлено klopik_core.py: word_to_pt3(), bucket query, morpho.
"""

import math
from math import gcd
from typing import Dict, List, Tuple, Optional
from collections import defaultdict


class CubeNode:
    """Узел 3D куба."""
    __slots__ = ('x', 'y', 'z', 'V', 'S', 'C', 'D_body', 'phase', 'disc')

    def __init__(self, x: int, y: int, z: int):
        self.x = x
        self.y = y
        self.z = z
        self.V = x * y * z           # Объём
        self.S = x + y + z           # Сумма
        self.C = x*y + x*z + y*z    # Планарная
        self.D_body = abs(x-y) + abs(y-z) + abs(x-z)  # Телесная разница
        self.phase = (x % 2) | ((y % 2) << 1) | ((z % 2) << 2)  # 3-bit octant
        self.disc = self._discriminant()

    def _discriminant(self) -> int:
        """Дискриминант кубического поля."""
        e1, e2, e3 = self.S, self.C, self.V
        return (18*e1*e2*e3 - 4*e1*e1*e1*e3 + e1*e1*e2*e2
                - 4*e2*e2*e2 - 27*e3*e3)

    def coords(self) -> Tuple[int, int, int]:
        return (self.x, self.y, self.z)

    def __repr__(self):
        return f"CubeNode({self.x},{self.y},{self.z}) V={self.V} S={self.S} D={self.D_body}"


class GenerativeCube:
    """3D куб с генеративным развёртыванием узлов.

    Принцип: не храним весь куб. Генерируем узел по адресу.
    Посещённые узлы кэшируются. Bucket index для поиска соседей.
    """

    def __init__(self, max_coord: int = 1024, bucket_size: int = 10):
        self.max_coord = max_coord
        self.bucket_size = bucket_size

        # Кэш посещённых узлов
        self._cache: Dict[Tuple[int, int, int], CubeNode] = {}

        # Bucket spatial index: (bx, by, bz) → список (x,y,z)
        self._buckets: Dict[Tuple[int, int, int], List[Tuple[int, int, int]]] = defaultdict(list)

        # Morpho-связи: (src_hash, tgt_hash) → weight
        self._morpho: Dict[Tuple[int, int], float] = defaultdict(float)

        # Seed table для генерации (используем 2D таблицу как seed)
        self._seed_cache: Dict[int, int] = {}

    def get_node(self, x: int, y: int, z: int) -> CubeNode:
        """Получить или сгенерировать узел по адресу. O(1)."""
        addr = (x, y, z)
        if addr in self._cache:
            return self._cache[addr]

        # Clamp
        x = max(0, min(self.max_coord, x))
        y = max(0, min(self.max_coord, y))
        z = max(0, min(self.max_coord, z))
        addr = (x, y, z)

        if addr in self._cache:
            return self._cache[addr]

        # Генерация узла
        node = CubeNode(x, y, z)
        self._cache[addr] = node

        # Добавить в bucket index
        bx, by, bz = x // self.bucket_size, y // self.bucket_size, z // self.bucket_size
        self._buckets[(bx, by, bz)].append(addr)

        return node

    def get_neighbors(self, x: int, y: int, z: int, radius: int) -> List[Tuple[int, CubeNode]]:
        """Найти соседей в радиусе через bucket index. O(N соседей)."""
        bs = self.bucket_size
        qb = (x // bs, y // bs, z // bs)
        sp = radius // bs + 1

        neighbors = []
        for dx in range(-sp, sp + 1):
            for dy in range(-sp, sp + 1):
                for dz in range(-sp, sp + 1):
                    bucket_key = (qb[0] + dx, qb[1] + dy, qb[2] + dz)
                    for addr in self._buckets.get(bucket_key, []):
                        nx, ny, nz = addr
                        dist = abs(x - nx) + abs(y - ny) + abs(z - nz)
                        if dist <= radius and addr != (x, y, z):
                            node = self.get_node(nx, ny, nz)
                            neighbors.append((dist, node))

        neighbors.sort(key=lambda p: p[0])
        return neighbors

    def morpho_link(self, src: CubeNode, tgt: CubeNode, weight: float = 1.0):
        """Создать или усилить морфо-связь."""
        key = (hash(src.coords()), hash(tgt.coords()))
        self._morpho[key] = max(self._morpho[key], weight)

    def get_morpho_neighbors(self, node: CubeNode) -> List[Tuple[CubeNode, float]]:
        """Получить морфо-соседей узла."""
        h = hash(node.coords())
        result = []
        for (src, tgt), w in self._morpho.items():
            if src == h:
                tgt_addr = self._find_addr_by_hash(tgt)
                if tgt_addr:
                    result.append((self.get_node(*tgt_addr), w))
            elif tgt == h:
                src_addr = self._find_addr_by_hash(src)
                if src_addr:
                    result.append((self.get_node(*src_addr), w))
        return result

    def _find_addr_by_hash(self, h: int) -> Optional[Tuple[int, int, int]]:
        """Найти адрес по хешу (для morpho)."""
        for addr, node in self._cache.items():
            if hash(addr) == h:
                return addr
        return None

    def tension(self, a: CubeNode, b: CubeNode) -> int:
        """Вычислить tension между двумя узлами."""
        dist = abs(a.x - b.x) + abs(a.y - b.y) + abs(a.z - b.z)
        phase_pen = bin(a.phase ^ b.phase).count('1') * 100
        shape_pen = abs(a.D_body - b.D_body)
        return dist + phase_pen + shape_pen

    def resonance(self, tension: int) -> float:
        """Резонанс: чем меньше tension, тем больше резонанс."""
        return 1.0 / math.log2(tension + 2.0)

    def analogy_3d(self, A: CubeNode, B: CubeNode, C: CubeNode) -> CubeNode:
        """Решить аналогию A:B :: C:D в 3D."""
        delta_x = B.x - A.x
        delta_y = B.y - A.y
        delta_z = B.z - A.z

        dx = C.x + delta_x
        dy = C.y + delta_y
        dz = C.z + delta_z

        return self.get_node(dx, dy, dz)

    def stats(self) -> Dict:
        """Статистика куба."""
        return {
            'cached_nodes': len(self._cache),
            'buckets': len(self._buckets),
            'morpho_links': len(self._morpho),
            'max_coord': self.max_coord,
            'address_space': self.max_coord ** 3,
        }


# ── Тесты ──

if __name__ == "__main__":
    print("=" * 60)
    print("  GenerativeCube — Тесты")
    print("=" * 60)

    cube = GenerativeCube(max_coord=1024)

    # Тест 1: Генерация узлов
    print("\n[Тест 1] Генерация узлов")
    n1 = cube.get_node(10, 20, 30)
    n2 = cube.get_node(10, 20, 30)
    print(f"  get_node(10,20,30) = {n1}")
    print(f"  get_node(10,20,30) = {n2}")
    assert n1 is n2, "Кэш должен возвращать тот же объект"
    print("  ✅ Кэш работает")

    # Тест 2: Соседи
    print("\n[Тест 2] Поиск соседей")
    # Создаём несколько узлов рядом
    for x in range(5, 15):
        for y in range(15, 25):
            for z in range(25, 35):
                cube.get_node(x, y, z)

    neighbors = cube.get_neighbors(10, 20, 30, radius=5)
    print(f"  Соседи (10,20,30) в радиусе 5: {len(neighbors)}")
    assert len(neighbors) > 0
    print("  ✅ Поиск соседей работает")

    # Тест 3: Tension
    print("\n[Тест 3] Tension")
    n1 = cube.get_node(10, 20, 30)
    n2 = cube.get_node(11, 21, 31)
    t = cube.tension(n1, n2)
    print(f"  tension((10,20,30), (11,21,31)) = {t}")
    assert t > 0
    print("  ✅ Tension работает")

    # Тест 4: Analogy
    print("\n[Тест 4] Analogy 3D")
    A = cube.get_node(1, 1, 1)
    B = cube.get_node(2, 2, 2)
    C = cube.get_node(3, 3, 3)
    D = cube.analogy_3d(A, B, C)
    print(f"  analogy((1,1,1), (2,2,2), (3,3,3)) = {D}")
    assert D.x == 4 and D.y == 4 and D.z == 4
    print("  ✅ Analogy работает")

    # Тест 5: Morpho
    print("\n[Тест 5] Morpho-связи")
    n1 = cube.get_node(100, 100, 100)
    n2 = cube.get_node(101, 101, 101)
    cube.morpho_link(n1, n2, weight=0.5)
    cube.morpho_link(n1, n2, weight=0.8)  # Усилить
    morpho_n = cube.get_morpho_neighbors(n1)
    print(f"  Morpho-соседи: {len(morpho_n)}")
    assert len(morpho_n) == 1
    print("  ✅ Morpho работает")

    # Тест 6: Статистика
    print("\n[Тест 6] Статистика")
    s = cube.stats()
    print(f"  {s}")
    assert s['cached_nodes'] > 0
    assert s['address_space'] == 1024**3
    print("  ✅ Статистика работает")

    print("\n" + "=" * 60)
    print("  ✅ ВСЕ ТЕСТЫ ПРОЙДЕНЫ")
    print("=" * 60)
    print(f"""
Генеративный куб:
  Адресное пространство: {s['address_space']:,} узлов
  Кэшировано: {s['cached_nodes']} узлов
  Память: ~{s['cached_nodes'] * 100} байт (вместо {s['address_space'] * 28:,} байт)
""")
