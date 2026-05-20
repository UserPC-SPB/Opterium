"""
swarm.py  —  Intelligent Swarm V2 (Opterium Ants)
Source: AI_BIOS.txt lines 197-198; bootloader.txt line 181

Replaces: Bayesian inference as primary decision engine.
Doctor uses this for routing: each agent is a hypothesis.
  P_jk(t) = [k1 + u_norm_j^α · (1+H_j) + k2 · (1/kol_j)^β + k3 · (kol_j/MaxKol_j)^γ]
            / Σ_z (...)

Components:
  μ^α       — exploitation (follow strong trails)
  (1/kol)^β — exploration (visit novel nodes)
  (kol/Max)^γ — potential (nodes useful in many contexts)
  H_j       — wisdom (historical success memory)

When Doctor chooses between routes, it runs the Swarm.
When you'd call Bayes, you call Swarm instead.
"""
from __future__ import annotations
import math, random
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple, TypeVar

T = TypeVar('T')

# ───────────────────────────────────────────────────────
# Node — one decision point
# ───────────────────────────────────────────────────────
class SwarmNode:
    """A node in the swarm graph. Carries local state."""
    __slots__ = ('id','_visits','_successes','_pheromone','_potential')
    def __init__(self, nid: str, potential: float = 1.0):
        self.id = nid
        self._visits = 0
        self._successes = 0
        self._pheromone = 1.0
        self._potential = max(0.01, float(potential))

    @property
    def visits(self) -> int: return self._visits
    @property
    def success_rate(self) -> float:
        return self._successes / max(1, self._visits)
    @property
    def pheromone(self) -> float: return self._pheromone
    @property
    def potential(self) -> float: return self._potential

    def reinforce(self, success: bool):
        self._visits += 1
        if success:
            self._successes += 1
        self._pheromone = self.success_rate * 0.7 + self._pheromone * 0.3

    def __repr__(self):
        return f"Node({self.id}: μ={self._pheromone:.3f}, H={self.success_rate:.3f}, kol={self._visits})"

# ───────────────────────────────────────────────────────
# Swarm — the decision engine that replaces Bayes
# ───────────────────────────────────────────────────────
class IntelligentSwarm:
    """
    P_jk(t) = [k1 + u_norm_j^α · (1+H_j) + k2 · (1/kol_j)^β + k3 · (kol_j/MaxKol_j)^γ]
              / Σ_z (...)

    Call .decide(context) → SwarmNode with highest probability.
    Call .update(node, success) → reinforce memory.
    """
    def __init__(self, k1: float = 1.0, k2: float = 2.0, k3: float = 1.5,
                 alpha: float = 2.0, beta: float = 1.5, gamma: float = 1.0,
                 temperature: float = 1.0, seed: int = 0):
        self.k1 = k1
        self.k2 = k2
        self.k3 = k3
        self.alpha = alpha
        self.beta = beta
        self.gamma = gamma
        self.temperature = temperature
        self.rng = random.Random(seed)
        self._nodes: Dict[str, SwarmNode] = {}
        self._history: Dict[str, float] = {}  # H_j persistence
        self._last_probs: Dict[str, float] = {}

    def register(self, nid: str, potential: float = 1.0) -> SwarmNode:
        if nid not in self._nodes:
            self._nodes[nid] = SwarmNode(nid, potential)
        return self._nodes[nid]

    def _u_norm(self, node: SwarmNode) -> float:
        """Normalized pheromone: μ_j / max μ across known nodes."""
        max_mu = max((n.pheromone for n in self._nodes.values()), default=1.0)
        return node.pheromone / max(max_mu, 0.01)

    def _h_factor(self, node: SwarmNode) -> float:
        """H_j = historical success index."""
        base = self._history.get(node.id, 0.0)
        return 1.0 + base

    def _kol_inv(self, node: SwarmNode) -> float:
        """1/kol_j — exploration drive."""
        return 1.0 / max(1, node.visits)

    def _kol_ratio(self, node: SwarmNode) -> float:
        """kol_j / MaxKol — potential/ pragmatism."""
        max_kol = max((n.visits for n in self._nodes.values()), default=1)
        return node.visits / max(max_kol, 1)

    def score(self, nid: str) -> float:
        """Compute P_j for a single node."""
        node = self._nodes.get(nid)
        if node is None:
            return 0.0
        u = self._u_norm(node)
        hi = self._h_factor(node)
        ki = self._kol_inv(node)
        kr = self._kol_ratio(node)
        return (self.k1
                + (u ** self.alpha) * hi
                + self.k2 * (ki ** self.beta)
                + self.k3 * (kr ** self.gamma))

    def probabilities(self) -> Dict[str, float]:
        """Full probability distribution over all nodes."""
        raw = {nid: self.score(nid) for nid in self._nodes}
        total = sum(raw.values())
        if total <= 0:
            return {nid: 1.0 / max(1, len(raw)) for nid in raw}
        probs = {nid: s / total for nid, s in raw.items()}
        self._last_probs = probs
        return probs

    def decide(self, candidates: Optional[List[str]]=None,
               deterministic: bool=False) -> SwarmNode:
        """Choose best node. deterministic=True → argmax, else temperature-sampled."""
        probs = self.probabilities()
        if candidates:
            probs = {k: probs.get(k, 0.0) for k in candidates}
            total = sum(probs.values())
            if total <= 0:
                probs = {k: 1.0 / max(1, len(candidates)) for k in candidates}
            else:
                probs = {k: v / total for k, v in probs.items()}

        if deterministic or self.temperature < 0.01:
            best = max(probs, key=probs.get)
            return self._nodes[best]

        items = list(probs.items())
        if self.temperature != 1.0:
            items = [(k, p ** (1.0 / self.temperature)) for k, p in items]
            total = sum(p for _, p in items)
            items = [(k, p / total) for k, p in items]

        r = self.rng.random()
        cum = 0.0
        for nid, p in items:
            cum += p
            if r < cum:
                return self._nodes[nid]
        return self._nodes[items[-1][0]]

    def update(self, nid: str, success: bool):
        """Reinforce node. Success → H_j increases."""
        node = self.register(nid)
        node.reinforce(success)
        old_h = self._history.get(nid, 0.0)
        delta = 0.1 if success else -0.05
        self._history[nid] = max(-0.5, min(2.0, old_h + delta))

    def __repr__(self):
        return f"Swarm({len(self._nodes)} nodes, T={self.temperature})"

# ───────────────────────────────────────────────────────
# Bayesian interface — calls Swarm under the hood
# ───────────────────────────────────────────────────────
class BayesReplacement:
    """Drops into code that expects P(hypothesis | evidence).
    Uses Intelligent Swarm instead of explicit Bayes rule."""
    def __init__(self, swarm: IntelligentSwarm):
        self.swarm = swarm

    def update_belief(self, hypothesis: str, evidence_weight: float):
        """Instead of P(E|H)·P(H), reinforce node proportionally."""
        success = evidence_weight > 0.5
        self.swarm.update(hypothesis, success)

    def posterior(self, candidates: List[str]) -> Dict[str, float]:
        """Returns P(H|E) ≈ swarm probabilities."""
        probs = self.swarm.probabilities()
        return {c: probs.get(c, 0.0) for c in candidates}

    def predict(self, candidates: List[str]) -> str:
        """Most likely hypothesis."""
        return self.swarm.decide(candidates, deterministic=True).id

# ───────────────────────────────────────────────────────
# Self-test
# ───────────────────────────────────────────────────────
def selftest():
    swarm = IntelligentSwarm(seed=42)
    for name in ['route_A', 'route_B', 'route_C', 'route_D']:
        swarm.register(name, potential=1.0)

    probs = swarm.probabilities()
    assert abs(sum(probs.values()) - 1.0) < 1e-12
    assert len(probs) == 4

    for _ in range(50):
        chosen = swarm.decide(deterministic=False)
        swarm.update(chosen.id, success=(chosen.id != 'route_C'))

    best = swarm.decide(deterministic=True)
    assert best.id != 'route_C'

    bayes = BayesReplacement(swarm)
    bayes.update_belief('route_A', 0.9)
    post = bayes.posterior(['route_A', 'route_B'])
    assert len(post) == 2
    assert bayes.predict(['route_A', 'route_B']) in ('route_A', 'route_B')

    print("  Swarm: all tests pass")

if __name__ == '__main__':
    selftest()
