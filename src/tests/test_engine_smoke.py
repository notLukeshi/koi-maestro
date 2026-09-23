"""Import + API smoke tests for the `koi_maestro._engine` extension.

Run under the installed wheel (`maturin build` + `pip install`), not the
source tree — the point is proving the shipped artifact exposes a working
surface: deal, legality, every solver backend, the observed-ledger
resolving contract, and end-to-end determinism.
"""

import pytest

from koi_maestro import _engine


def test_module_surface():
    assert _engine.__version__
    for name in (
        "Action",
        "CaptureChoice",
        "Card",
        "GameState",
        "IsmctsSettings",
        "LedgerEntry",
        "PimcSettings",
        "Player",
        "PublicObservation",
        "PublicView",
        "ResolvingSettings",
        "Solver",
    ):
        assert hasattr(_engine, name), name


def test_deal_and_legal_actions():
    state = _engine.GameState.deal(42)
    assert state.phase == "hand"
    assert state.turn == 0
    assert len(state.public_view(_engine.Player.South).own_hand) == 8
    assert len(state.public_view(_engine.Player.North).own_hand) == 8
    assert len(state.field) == 8
    actions = state.legal_actions()
    assert actions and all(action.kind == "play_from_hand" for action in actions)


def test_action_key_orders_stably():
    state = _engine.GameState.deal(42)
    keys = [action.action_key for action in state.legal_actions()]
    assert keys == sorted(keys)


def test_public_view_hides_opponent_hand():
    state = _engine.GameState.deal(42)
    view = state.public_view(_engine.Player.South)
    assert view.opponent_hand_count == 8
    assert view.observer == _engine.Player.South


def test_invalid_card_index_rejected():
    with pytest.raises(ValueError):
        _engine.Card(48)


@pytest.mark.parametrize("method", ["random", "heuristic", "ismcts", "pimc", "endgame"])
def test_backend_picks_legal_action(method):
    state = _engine.GameState.deal(42)
    solver = _engine.Solver(method, seed=7)
    if method == "ismcts":
        solver.ismcts.iterations = 200
    if method == "pimc":
        solver.pimc.max_simulations = 8
    solver.build()
    action = solver.find_best_action(state)
    assert action is not None
    assert action in state.legal_actions()


def test_archetype_backend():
    state = _engine.GameState.deal(42)
    solver = _engine.Solver("archetype", archetype="banker", seed=7)
    solver.build()
    action = solver.find_best_action(state)
    assert action is not None


def test_resolving_requires_observation():
    state = _engine.GameState.deal(42)
    solver = _engine.Solver("resolving", seed=7)
    solver.resolving.max_worlds = 4
    solver.resolving.cfr_iterations = 10
    solver.build()
    with pytest.raises(ValueError):
        solver.find_best_action(state)


def test_resolving_consumes_ledger():
    """A full resolving leg: the ledger is rebuilt per decision exactly as
    the referee ships it, so every resolve replays real history."""
    state = _engine.GameState.deal(42)
    solver = _engine.Solver("resolving", seed=7)
    solver.resolving.max_worlds = 4
    solver.resolving.max_nodes = 20_000
    solver.resolving.cfr_iterations = 10
    solver.resolving.max_decision_depth = 2
    solver.build()

    observer = _engine.Player.South
    ledger = _engine.PublicObservation(state.public_view(observer))
    heuristic = _engine.Solver("heuristic", seed=7)
    heuristic.build()
    turns = 0
    while not state.is_ended and turns < 64:
        actor = state.active
        if actor == observer:
            action = solver.find_best_action(state, ledger, solver_seed=1000 + turns)
        else:
            action = heuristic.find_best_action(state)
        assert action is not None
        drawn = state.drawn if action.kind == "resolve_stock" else None
        ledger.push(_engine.LedgerEntry(actor, action, drawn))
        state.apply_action(action)
        turns += 1
    assert turns > 0


def test_determinism_same_seed():
    def run_once():
        state = _engine.GameState.deal(42)
        solver = _engine.Solver("ismcts", seed=5)
        solver.ismcts.iterations = 100
        solver.build()
        return solver.find_best_action(state, solver_seed=11)

    assert run_once() == run_once()


def test_next_round_carries_match_state():
    state = _engine.GameState.deal(42)
    heuristic = _engine.Solver("heuristic", seed=7)
    heuristic.build()
    guard = 0
    while not state.is_ended and guard < 128:
        action = heuristic.find_best_action(state)
        if action is None:
            break
        state.apply_action(action)
        guard += 1
    assert state.is_ended
    nxt = state.next_round(seed=43)
    assert nxt.turn == 0
    # `round` is the next-round index — settlement already advanced it, so
    # next_round carries the same value forward rather than adding one.
    assert nxt.round == state.round
    assert nxt.score == state.score


if __name__ == "__main__":
    import sys

    sys.exit(pytest.main([__file__, "-q"]))
