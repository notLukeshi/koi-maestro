"""koi-maestro quickstart — a guided tour of the engine API.

Run this file against the installed wheel:

    pip install .            # or: maturin develop
    python examples/quickstart.py

Every section is executable top to bottom and prints what it does, so the
console output doubles as the lesson. The point is not to demo features —
it is to show *how the engine thinks*: what a state is, what a solver is
allowed to see, and why some backends need an observation ledger.
"""

from koi_maestro import _engine


def seat_name(player: "_engine.Player") -> str:
    return str(player).rsplit(".", 1)[-1]  # pyo3 enums have no .name


def banner(title: str) -> None:
    print(f"\n=== {title} " + "=" * (64 - len(title)))


def card_name(card: "_engine.Card") -> str:
    """Human-readable card name: "Aug-Moon", "Sep-SakeCup", "Jan-kasu2".

    Cards are plain integers `month * 4 + index`; the engine deliberately
    exposes only structure, so names are a presentation-layer concern.
    """
    months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
    # index 0 is the month's Hikari/Tane (SakeCup in September); the
    # Tanzaku ribbon is index 1 — except November, where the Swallow
    # (tane) is index 1 and the ribbon sits at index 2; December has none.
    specials = {
        (0, 0): "Crane",
        (2, 0): "Curtain",
        (7, 0): "Moon",
        (10, 0): "RainMan",
        (11, 0): "Phoenix",  # Hikari
        (8, 0): "SakeCup",  # counts as Tane
        (1, 0): "BushWarbler",
        (3, 0): "Cuckoo",
        (4, 0): "Bridge",
        (5, 0): "Butterflies",
        (6, 0): "Boar",
        (7, 1): "Geese",
        (9, 0): "Deer",
        (10, 1): "Swallow",  # Tane
    }
    key = (card.month, card.index_in_month)
    if key in specials:
        return f"{months[key[0]]}-{specials[key]}"
    ribbon_idx = 2 if key[0] == 10 else 1
    if key[0] != 11 and key[1] == ribbon_idx:
        return f"{months[key[0]]}-ribbon"
    return f"{months[key[0]]}-kasu{key[1]}"


def action_desc(action: "_engine.Action") -> str:
    """One readable line for an action — raw reprs show Rust internals."""
    cap = action.capture
    cap_kind = cap.kind if cap is not None else "no_match"
    if cap_kind == "triple":  # captures all three same-month cards
        tgt = " -> all same-month cards"
    elif cap is not None and cap.card is not None:
        tgt = f" -> {card_name(cap.card)}"
    else:
        tgt = " (no match)"
    if action.kind == "play_from_hand":
        return f"play {card_name(action.card)}{tgt}"
    if action.kind == "resolve_stock":
        return f"stock{tgt}"
    return action.kind  # "koi_koi" / "shobu"


# ── 1. Dealing a round ──────────────────────────────────────────────────
banner("1 · Dealing")
#
# A *round* (one hand of Koi-Koi) is dealt from a single u64 seed — the
# same seed always produces the same deal, which is what makes the
# benchmark harness reproducible. Anomalies (Teshi instant wins, void
# fields) are resolved inside the constructor, exactly like the referee.
state = _engine.GameState.deal(seed=42, ruleset="nintendo")

print(f"phase={state.phase}  turn={state.turn}  dealer={seat_name(state.dealer)}")
print("field:", ", ".join(card_name(c) for c in state.field))
print("south hand:", ", ".join(card_name(c) for c in state.hand(_engine.Player.South)))
print("stock:", state.stock_count, "cards")

# ── 2. The two ways to look at a state ──────────────────────────────────
banner("2 · Full state vs public view")
#
# `state.hand(player)` is *full information* — referee eyes. A solver is
# not allowed that: it must reason from `public_view(seat)`, which hides
# the opponent's hand (a count, never the cards) and the stock order.
# This boundary is what makes Koi-Koi an imperfect-information game.
view = state.public_view(_engine.Player.South)
print(
    f"observer={seat_name(view.observer)}  own={len(view.own_hand)} "
    f"opp_hand={view.opponent_hand_count} (count only)  field={len(view.field)}"
)

# ── 3. Legal actions ────────────────────────────────────────────────────
banner("3 · Legal actions")
#
# The kernel is the single source of truth for legality — UIs and solvers
# enumerate `legal_actions()` and pick one; they never construct moves by
# hand. `apply_action` mutates in place and rejects illegal transitions.
for a in state.legal_actions():
    print(" ", action_desc(a))

if state.is_ended:
    # rare but real: a deal anomaly (Teshi/void) can end a round instantly
    print("deal resolved instantly — redealing for the demo")
    state = _engine.GameState.deal(seed=43)
first = state.legal_actions()[0]
state.apply_action(first)
print(
    f"\napplied '{action_desc(first)}' -> phase={state.phase} "
    f"(a turn is hand-play then stock-resolution)"
)

# ── 4. The solver zoo ───────────────────────────────────────────────────
banner("4 · Solvers")
#
# One facade, eight backends. `Solver(method)` + `build()` + repeated
# `find_best_action(state)`. Weakest to strongest measured field entrant:
#
#   random       uniform legal pick — the sanity floor
#   archetype    scripted personalities: "materialist", "yaku_chaser",
#                "banker", "gambler", "timid" — cheap, human-flavoured
#   heuristic    handcrafted greedy evaluator — fast, no search
#   ismcts       information-set MCTS: samples plausible hidden worlds,
#                runs UCB tree search per world (`ismcts.iterations`)
#   pimc         perfect-information Monte Carlo: samples determinizations
#                and evaluates each to the end (`pimc.max_simulations`,
#                `pimc.scoring_method` = "mean" margin or "win_rate")
#   endgame      exact turn-8 solver + turn-7 LP subgame — optimal when
#                it applies, falls back to a heuristic mid-game
#   leaf_policy  the ONNX policy network used directly, no search —
#                set `solver.leaf_model` to a model path, or None for the
#                deterministic handcrafted evaluator
#   resolving    continual resolving: maintains a belief over the
#                opponent's hand (`resolving.max_worlds`), compiles a
#                gadget subgame (`max_nodes`, `max_decision_depth`) and
#                runs CFR+ on it (`cfr_iterations`). `adaptive=True` adds
#                an opponent-model posterior; `ox=True` an exploitation
#                arm (requires adaptive). `leaf_model` prices subgame
#                leaves: None = margin oracle, "builtin:handcrafted" =
#                deterministic evaluator, or an .onnx path (leaf-ort).
#
# The tournament champion configuration is resolving + the ONNX leaf:
# ~10x less compute than plain resolving, stronger measured play.

fast = _engine.Solver("heuristic", seed=1)
fast.build()
print("heuristic chose:", action_desc(fast.find_best_action(state)))

thinker = _engine.Solver("pimc", seed=2)
thinker.pimc.max_simulations = 128  # more worlds = slower, sharper
thinker.pimc.scoring_method = "mean"  # optimize point margin, not just W/L
thinker.build()  # settings freeze at build time
print("pimc chose:", action_desc(thinker.find_best_action(state)))

# ── 5. The observation ledger (resolving only) ──────────────────────────
banner("5 · Observation ledger")
#
# `resolving` is the only backend whose input is not the state but the
# *observation*: what the attested seat could legally have seen — the
# deal-time view plus every applied action, in order. You build it once
# per round and push a LedgerEntry after every `apply_action` — for BOTH
# players, since every action is public. Forgetting a push (or the drawn
# card on a resolve_stock entry) corrupts the belief; omitting the
# observation entirely fails loudly rather than guessing.
seat = _engine.Player.South
demo = _engine.GameState.deal(seed=42)
obs = _engine.PublicObservation(demo.public_view(seat))  # built at deal time
res = _engine.Solver("resolving", seed=3)
res.resolving.max_worlds = 12
res.resolving.cfr_iterations = 80
res.resolving.leaf_model = "builtin:handcrafted"  # no ONNX needed
res.build()
opp = _engine.Solver("heuristic", seed=4)
opp.build()

for _ in range(4):  # a few plies of demo
    if demo.is_ended:
        break
    actor = demo.active  # capture BEFORE apply
    if actor == seat:
        a = res.find_best_action(demo, observation=obs)
    else:
        a = opp.find_best_action(demo)  # plain solvers need none
    if a is None:  # a solver may abstain; fall back to a legal move
        a = demo.legal_actions()[0]
    drawn = demo.drawn if a.kind == "resolve_stock" else None
    demo.apply_action(a)
    obs.push(_engine.LedgerEntry(actor, a, drawn))
print(f"ledger: {len(obs)} public entries, phase={demo.phase}")

# ── 6. A full round, solver vs solver ───────────────────────────────────
banner("6 · Full leg simulation")


#
# A *leg* is one round. The harness always plays mirrored pairs (same deal
# seed, swapped seats) so deal luck cancels — we mirror that here with two
# legs on one seed.
def play_leg(seed: int, south: _engine.Solver, north: _engine.Solver) -> int:
    """Play one round; return South's leg margin."""
    s = _engine.GameState.deal(seed)
    while not s.is_ended:  # Player enums are unhashable —
        cur = south if s.active == _engine.Player.South else north  # so compare
        a = cur.find_best_action(s)
        if a is None:  # safety: engine never deadlocks
            a = s.legal_actions()[0]  # a legal move always exists
        s.apply_action(a)
    return s.leg_margin(_engine.Player.South)


p1 = _engine.Solver("pimc", seed=11)
p1.pimc.max_simulations = 64
p1.build()
p2 = _engine.Solver("heuristic", seed=12)
p2.build()

m1 = play_leg(777, p1, p2)  # p1 deals as South
m2 = play_leg(777, p2, p1)  # mirrored: same seed, swapped seats
print(
    f"seed 777 — pimc as South {m1:+d}, pimc as North {-m2:+d} "
    f"(pair margin {m1 - m2:+d} cancels deal luck)"
)

# ── 7. What the engine guarantees ───────────────────────────────────────
banner("7 · Guarantees")
#
# * legal_actions() is never empty mid-round — every phase (hand, stock,
#   stop) always admits at least one move; the round ends when hands and
#   stock exhaust or someone calls shobu.
# * deal(seed) is deterministic end-to-end: same seed + same solver seeds
#   → same leg, bit for bit.
# * public_view/ledger never leak hidden cards — solvers cannot cheat.
# * state.next_round(seed) continues a match: dealer rotates to the round
#   winner and the score carries over.
print("done — next: examples/play_koikoi.py for an interactive game")
