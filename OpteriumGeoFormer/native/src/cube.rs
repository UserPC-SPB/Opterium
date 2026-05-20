//! cube.rs — Generative 3D Cube with on-demand node generation.
//!
//! Не хранит весь куб (1024³ = 1B узлов = 28GB).
//! Генерирует узел по адресу: O(1).
//! Bucket spatial index для поиска соседей.
//! Morpho-связи (живые, усиливаются).

use std::collections::HashMap;

/// 3D узел куба.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubeNode {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub v: i64,       // Объём: x*y*z
    pub s: i32,       // Сумма: x+y+z
    pub c: i64,       // Планарная: xy+xz+yz
    pub d_body: i32,  // Телесная разница: |x-y|+|y-z|+|x-z|
    pub phase: u8,    // 3-bit octant: (x%2)|(y%2<<1)|(z%2<<2)
    pub disc: i64,    // Дискриминант кубического поля
}

impl CubeNode {
    /// Создать узел из координат.
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        let v = x as i64 * y as i64 * z as i64;
        let s = x + y + z;
        let c = x as i64 * y as i64 + x as i64 * z as i64 + y as i64 * z as i64;
        let d_body = (x - y).abs() + (y - z).abs() + (x - z).abs();
        let phase = ((x & 1) | ((y & 1) << 1) | ((z & 1) << 2)) as u8;
        let disc = Self::discriminant(s, c, v);

        Self { x, y, z, v, s, c, d_body, phase, disc }
    }

    /// Дискриминант кубического поля.
    fn discriminant(e1: i32, e2: i64, e3: i64) -> i64 {
        let e1 = e1 as i64;
        18 * e1 * e2 * e3
            - 4 * e1 * e1 * e1 * e3
            + e1 * e1 * e2 * e2
            - 4 * e2 * e2 * e2
            - 27 * e3 * e3
    }

    /// Координаты как кортеж.
    pub fn coords(&self) -> (i32, i32, i32) {
        (self.x, self.y, self.z)
    }

    /// Хеш адреса.
    pub fn addr_hash(&self) -> u64 {
        // FNV-1a hash
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &[self.x as u32, self.y as u32, self.z as u32] {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// Bucket spatial index: (bx, by, bz) → список адресов
type BucketIndex = HashMap<(i32, i32, i32), Vec<(i32, i32, i32)>>;

/// Morpho-связь: (src_hash, tgt_hash) → weight
type MorphoLinks = HashMap<(u64, u64), f64>;

/// Generative 3D Cube.
pub struct GenerativeCube {
    max_coord: i32,
    bucket_size: i32,
    cache: HashMap<(i32, i32, i32), CubeNode>,
    buckets: BucketIndex,
    morpho: MorphoLinks,
}

impl GenerativeCube {
    /// Создать новый куб.
    pub fn new(max_coord: i32, bucket_size: i32) -> Self {
        Self {
            max_coord,
            bucket_size,
            cache: HashMap::new(),
            buckets: HashMap::new(),
            morpho: HashMap::new(),
        }
    }

    /// Получить или сгенерировать узел по адресу. O(1).
    pub fn get_node(&mut self, x: i32, y: i32, z: i32) -> CubeNode {
        let x = x.clamp(0, self.max_coord);
        let y = y.clamp(0, self.max_coord);
        let z = z.clamp(0, self.max_coord);
        let addr = (x, y, z);

        if let Some(&node) = self.cache.get(&addr) {
            return node;
        }

        let node = CubeNode::new(x, y, z);
        self.cache.insert(addr, node);

        // Добавить в bucket index
        let bx = x / self.bucket_size;
        let by = y / self.bucket_size;
        let bz = z / self.bucket_size;
        self.buckets.entry((bx, by, bz)).or_insert_with(Vec::new).push(addr);

        node
    }

    /// Найти соседей в радиусе через bucket index.
    pub fn get_neighbors(&self, x: i32, y: i32, z: i32, radius: i32) -> Vec<(i32, CubeNode)> {
        let bs = self.bucket_size;
        let qb = (x / bs, y / bs, z / bs);
        let sp = radius / bs + 1;
        let target = (x, y, z);

        let mut neighbors = Vec::new();

        for dx in -sp..=sp {
            for dy in -sp..=sp {
                for dz in -sp..=sp {
                    let bucket_key = (qb.0 + dx, qb.1 + dy, qb.2 + dz);
                    if let Some(addrs) = self.buckets.get(&bucket_key) {
                        for &addr in addrs {
                            if addr == target {
                                continue;
                            }
                            let dist = (x - addr.0).abs() + (y - addr.1).abs() + (z - addr.2).abs();
                            if dist <= radius {
                                if let Some(&node) = self.cache.get(&addr) {
                                    neighbors.push((dist, node));
                                }
                            }
                        }
                    }
                }
            }
        }

        neighbors.sort_by_key(|&(d, _)| d);
        neighbors
    }

    /// Создать или усилить морфо-связь.
    pub fn morpho_link(&mut self, src: &CubeNode, tgt: &CubeNode, weight: f64) {
        let key = (src.addr_hash(), tgt.addr_hash());
        let entry = self.morpho.entry(key).or_insert(0.0);
        *entry = entry.max(weight);
    }

    /// Получить морфо-соседей узла.
    pub fn get_morpho_neighbors(&self, node: &CubeNode) -> Vec<(CubeNode, f64)> {
        let h = node.addr_hash();
        let mut result = Vec::new();

        for ((src, tgt), &w) in &self.morpho {
            if *src == h {
                if let Some(addr) = self.find_addr_by_hash(*tgt) {
                    if let Some(&n) = self.cache.get(&addr) {
                        result.push((n, w));
                    }
                }
            } else if *tgt == h {
                if let Some(addr) = self.find_addr_by_hash(*src) {
                    if let Some(&n) = self.cache.get(&addr) {
                        result.push((n, w));
                    }
                }
            }
        }

        result
    }

    /// Найти адрес по хешу.
    fn find_addr_by_hash(&self, h: u64) -> Option<(i32, i32, i32)> {
        for (&addr, node) in &self.cache {
            if node.addr_hash() == h {
                return Some(addr);
            }
        }
        None
    }

    /// Вычислить tension между двумя узлами.
    pub fn tension(&self, a: &CubeNode, b: &CubeNode) -> i32 {
        let dist = (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs();
        let phase_pen = (a.phase ^ b.phase).count_ones() as i32 * 100;
        let shape_pen = (a.d_body - b.d_body).abs();
        dist + phase_pen + shape_pen
    }

    /// Резонанс: чем меньше tension, тем больше резонанс.
    pub fn resonance(&self, tension: i32) -> f64 {
        1.0 / ((tension as f64 + 2.0).log2())
    }

    /// Решить аналогию A:B :: C:D в 3D.
    pub fn analogy_3d(&mut self, a: &CubeNode, b: &CubeNode, c: &CubeNode) -> CubeNode {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let dz = b.z - a.z;

        let nx = c.x + dx;
        let ny = c.y + dy;
        let nz = c.z + dz;

        self.get_node(nx, ny, nz)
    }

    /// Статистика куба.
    pub fn stats(&self) -> CubeStats {
        CubeStats {
            cached_nodes: self.cache.len(),
            buckets: self.buckets.len(),
            morpho_links: self.morpho.len(),
            max_coord: self.max_coord,
            address_space: (self.max_coord as i64).pow(3),
        }
    }
}

/// Статистика куба.
pub struct CubeStats {
    pub cached_nodes: usize,
    pub buckets: usize,
    pub morpho_links: usize,
    pub max_coord: i32,
    pub address_space: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_node_new() {
        let n = CubeNode::new(10, 20, 30);
        assert_eq!(n.x, 10);
        assert_eq!(n.y, 20);
        assert_eq!(n.z, 30);
        assert_eq!(n.v, 6000);
        assert_eq!(n.s, 60);
        assert_eq!(n.d_body, 40);
        assert_eq!(n.phase, 0); // 10%2=0, 20%2=0, 30%2=0
    }

    #[test]
    fn test_cube_node_phase() {
        let n = CubeNode::new(1, 0, 1);
        assert_eq!(n.phase, 0b101); // 1|0|4 = 5
    }

    #[test]
    fn test_get_node_cached() {
        let mut cube = GenerativeCube::new(1024, 10);
        let n1 = cube.get_node(10, 20, 30);
        let n2 = cube.get_node(10, 20, 30);
        assert_eq!(n1, n2);
    }

    #[test]
    fn test_get_neighbors() {
        let mut cube = GenerativeCube::new(1024, 10);
        for x in 5..15 {
            for y in 15..25 {
                for z in 25..35 {
                    cube.get_node(x, y, z);
                }
            }
        }
        let neighbors = cube.get_neighbors(10, 20, 30, 5);
        assert!(!neighbors.is_empty());
    }

    #[test]
    fn test_tension() {
        let mut cube = GenerativeCube::new(1024, 10);
        let n1 = cube.get_node(10, 20, 30);
        let n2 = cube.get_node(11, 21, 31);
        let t = cube.tension(&n1, &n2);
        assert!(t > 0);
    }

    #[test]
    fn test_analogy_3d() {
        let mut cube = GenerativeCube::new(1024, 10);
        let a = cube.get_node(1, 1, 1);
        let b = cube.get_node(2, 2, 2);
        let c = cube.get_node(3, 3, 3);
        let d = cube.analogy_3d(&a, &b, &c);
        assert_eq!(d.x, 4);
        assert_eq!(d.y, 4);
        assert_eq!(d.z, 4);
    }

    #[test]
    fn test_morpho_link() {
        let mut cube = GenerativeCube::new(1024, 10);
        let n1 = cube.get_node(100, 100, 100);
        let n2 = cube.get_node(101, 101, 101);
        cube.morpho_link(&n1, &n2, 0.5);
        cube.morpho_link(&n1, &n2, 0.8);
        let neighbors = cube.get_morpho_neighbors(&n1);
        assert_eq!(neighbors.len(), 1);
        assert!((neighbors[0].1 - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_stats() {
        let mut cube = GenerativeCube::new(1024, 10);
        cube.get_node(1, 2, 3);
        cube.get_node(4, 5, 6);
        let s = cube.stats();
        assert_eq!(s.cached_nodes, 2);
        assert_eq!(s.address_space, 1024i64.pow(3));
    }
}
