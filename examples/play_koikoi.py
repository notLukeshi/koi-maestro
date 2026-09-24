"""play_koikoi — a full terminal Koi-Koi game against the engine's solvers.

Requires Textual (not a package dependency — the wheel stays lean):

    pip install textual
    pip install .                    # the koi_maestro engine itself
    python examples/play_koikoi.py

Design notes
------------
The UI never invents moves: every interactive choice enumerates exactly
`GameState.legal_actions()`, so the game cannot deadlock or emit an
illegal transition — when a state admits one option (stock resolution,
forced captures) it is presented as such, and the engine is the only
arbiter of legality.

Solver moves run on a worker thread (`@work`) so the UI stays alive while
a heavy backend thinks. `resolving` additionally needs the public
observation ledger — one `PublicObservation` per round, fed a
`LedgerEntry` after *every* applied action, whichever seat acted.
"""

from __future__ import annotations

import importlib.util
import os
import random
from pathlib import Path
from typing import ClassVar

try:
    from rich.text import Text
    from textual import work
    from textual.app import App, ComposeResult
    from textual.binding import Binding
    from textual.containers import Horizontal, HorizontalGroup, Vertical, VerticalScroll
    from textual.css.query import NoMatches
    from textual.screen import ModalScreen, Screen
    from textual.widgets import (
        Button,
        Checkbox,
        Footer,
        Input,
        Label,
        LoadingIndicator,
        OptionList,
        Rule,
        Static,
    )
    from textual.widgets.option_list import Option
except ImportError:  # pragma: no cover - friendlier than a traceback
    raise SystemExit("play_koikoi needs Textual:  pip install textual")

from koi_maestro import _engine

# Leaf-backed solvers need the optional `leaf-ort` build feature plus an
# ONNX Runtime shared library discovered at runtime (ORT_DYLIB_PATH or
# the loader path). When the `onnxruntime` pip package is installed its
# bundled shared library is exactly what the loader needs — point
# ORT_DYLIB_PATH at it so a leaf-ort engine gets the real neural
# evaluator with zero manual configuration. Silently skipped otherwise:
# those tiers then degrade to the handcrafted evaluator.
if not os.environ.get("ORT_DYLIB_PATH"):
    try:
        _spec = importlib.util.find_spec("onnxruntime")
    except ImportError:
        _spec = None
    _capi = Path(_spec.origin).parent / "capi" if _spec and _spec.origin else None
    if _capi is not None and _capi.is_dir():
        _dll = next(
            (p for p in _capi.iterdir() if p.name.startswith(("onnxruntime.", "libonnxruntime"))),
            None,
        )
        if _dll is not None:
            os.environ["ORT_DYLIB_PATH"] = str(_dll)

# ── Card metadata ─────────────────────────────────────────────────────────
# The engine exposes month + index_in_month only; names/ranks are a
# presentation concern. Per month, index 0 is the Hikari or Tane (the
# September Sake Cup), the Tanzaku ribbon is index 1 (index 2 for
# November), and the rest are Kasu — mirroring
# koi_core::yaku::CARD_ATTRIBUTES.

MONTHS = [
    "Pine",
    "Plum",
    "Cherry",
    "Wisteria",
    "Iris",
    "Peony",
    "BushClover",
    "Pampas",
    "Chrysanth",
    "Maple",
    "Willow",
    "Paulownia",
]
# Distinct 4-char abbreviations for the card tiles (full names in action text)
MONTHS_SHORT = [
    "Pine",
    "Plum",
    "Cher",
    "Wist",
    "Iris",
    "Peon",
    "Bush",
    "Pamp",
    "Chry",
    "Mapl",
    "Will",
    "Paul",
]

SPECIALS = {
    (0, 0): ("Crane", "hikari"),
    (2, 0): ("Curtain", "hikari"),
    (7, 0): ("Moon", "hikari"),
    (10, 0): ("Rain Man", "hikari"),
    (11, 0): ("Phoenix", "hikari"),
    (8, 0): ("Sake Cup", "tane"),
    (1, 0): ("Warbler", "tane"),
    (3, 0): ("Cuckoo", "tane"),
    (4, 0): ("Bridge", "tane"),
    (5, 0): ("Butterflies", "tane"),
    (6, 0): ("Boar", "tane"),
    (7, 1): ("Geese", "tane"),
    (9, 0): ("Deer", "tane"),
    (10, 1): ("Swallow", "tane"),
}
BLUE_RIBBONS = {5, 8, 9}  # Jun, Sep, Oct


def ribbon_index(month: int) -> int:
    """A month's Tanzaku index: Nov's ribbon sits at index 2 (index 1 is
    the Swallow tane); Paulownia has no ribbon at all."""
    if month == 10:
        return 2
    if month == 11:
        return -1
    return 1


def card_label(card: _engine.Card) -> tuple[str, str, str]:
    """(month, name, rank-class) for a card."""
    key = (card.month, card.index_in_month)
    if key in SPECIALS:
        name, rank = SPECIALS[key]
    elif key[1] == ribbon_index(key[0]) and key[1] >= 0:
        name, rank = "Ribbon", "tanzaku-blue" if key[0] in BLUE_RIBBONS else "tanzaku"
    else:
        name, rank = "", "kasu"
    return MONTHS[key[0]], name, rank


def action_rich(action: _engine.Action) -> Text:
    """One aligned, colour-coded line per legal action:
    `▸ Play Plum Warbler    →  captures Plum Ribbon` — the fixed-width
    left column keeps every option scannable at a glance."""
    cap = action.capture
    cap_kind = cap.kind if cap is not None else "no_match"
    if action.kind == "koi_koi":
        return Text("» Koi-Koi   — keep playing for a bigger score", style="bold #ff5f5f")
    if action.kind == "shobu":
        return Text("» Shobu     — take the points, end the round", style="bold #7fd77f")
    if action.kind == "play_from_hand":
        m, n, _ = card_label(action.card)
        left = f"Play {m} {n or 'plain'}"
    elif action.kind == "resolve_stock":
        left = "Resolve drawn card"
    else:
        return Text(action.kind)  # unreachable under the current kernel
    if cap_kind == "triple":
        cm = MONTHS[action.card.month] if action.card is not None else "?"
        outcome = f"captures all {cm} cards"
    elif cap is not None and cap.card is not None:
        cm, cn, _ = card_label(cap.card)
        outcome = f"captures {cm} {cn or 'plain'}"
    else:
        outcome = "leaves it on the field"
    t = Text()
    t.append(f"» {left:<26}")
    t.append("→ ", style="cyan")
    t.append(outcome, style="#7fd77f" if outcome.startswith("captures") else "#888888")
    return t


# ── Opponent factory ──────────────────────────────────────────────────────
# Difficulty tiers mirror the measured tournament entrants. Heavier
# backends think longer per move — the label is honest about it.

MODEL_PATH = (
    Path(__file__).resolve().parent.parent
    / "crates"
    / "koi-solver"
    / "tests"
    / "leaf"
    / "model.onnx"
)
LEAF_MODEL_PATH = str(MODEL_PATH) if MODEL_PATH.exists() else None
# `resolving.leaf_model` also accepts the sentinel "builtin:handcrafted";
# `leaf_policy` instead uses None for the same deterministic evaluator.
RESOLVING_LEAF = LEAF_MODEL_PATH or "builtin:handcrafted"

# Short one-line labels keep the tier list compact (no wrapped options, no
# scroll) — the full backend name is what `build_solver` consumes anyway.
TIERS = [
    ("Novice", "heuristic, instant"),
    ("Casual", "ismcts, 2k worlds"),
    ("Skilled", "pimc, 256 sims"),
    ("Expert", "leaf_policy net"),
    ("Champion", "resolving, ~1s"),
]


def build_solver(method: str, seed: int, **over) -> _engine.Solver:
    """Build one solver; per-method settings applied before build().

    Aliases are normalized once (the engine accepts e.g.
    "resolving_adaptive" case-insensitively), and any leftover `over`
    key raises — a typo'd setting must not silently do nothing.
    """
    method = method.strip().lower()
    if method not in KNOWN_METHODS:
        raise ValueError(f"unknown solver method '{method}'")
    # resolving_adaptive = resolving with the adaptive+OX exploit machinery on
    # (the manifest's resolving_adaptive entrant); plain resolving defaults off.
    adaptive_default = method == "resolving_adaptive"
    if method.startswith("resolving"):
        method = "resolving"
    elif method in ("leafpolicy", "leaf"):
        method = "leaf_policy"
    s = _engine.Solver(method, seed=seed, archetype=over.pop("archetype", None))
    if method == "ismcts":
        s.ismcts.iterations = over.pop("iterations", 2000)
    elif method == "pimc":
        s.pimc.max_simulations = over.pop("max_simulations", 256)
        s.pimc.scoring_method = over.pop("scoring_method", "mean")
    elif method == "resolving":
        s.resolving.max_worlds = over.pop("max_worlds", 12)
        s.resolving.max_nodes = over.pop("max_nodes", 60_000)
        s.resolving.cfr_iterations = over.pop("cfr_iterations", 80)
        s.resolving.max_decision_depth = over.pop("max_decision_depth", 4)
        # adaptive=False + uniform belief is the measured winning spec
        # (resolving_leaf); the engine default is True.
        s.resolving.adaptive = over.pop("adaptive", adaptive_default)
        s.resolving.ox = over.pop("ox", adaptive_default)
        s.resolving.leaf_model = over.pop("leaf_model", RESOLVING_LEAF)
    elif method == "leaf_policy":
        s.leaf_model = over.pop("leaf_model", LEAF_MODEL_PATH)
    if over:
        raise ValueError(f"unknown settings for {method}: {sorted(over)}")
    s.build()
    return s


def probe_solver(solver: _engine.Solver, ruleset: str) -> None:
    """One real move on a fresh deal — catches failures that `build()`
    defers to move time (e.g. a leaf ONNX path without the leaf-ort
    build feature). Cheap for every backend except resolving (~1s).
    The ledger gets one real entry first: resolving refuses an empty
    public observation, which is correct — it has nothing to resolve."""
    st = _engine.GameState.deal(1, ruleset)
    obs = _engine.PublicObservation(st.public_view(_engine.Player.North))
    # advance to a real decision (>1 legal action): a forced stock
    # resolution returns without ever touching the leaf evaluator, so
    # probing there proves nothing
    for _ in range(40):
        if st.is_ended:
            return
        actions = st.legal_actions()
        if len(actions) > 1 and len(obs) > 0:
            break
        actor = st.active
        a = actions[0]
        drawn = st.drawn if a.kind == "resolve_stock" else None
        st.apply_action(a)
        obs.push(_engine.LedgerEntry(actor, a, drawn))
    if st.is_ended:
        return
    solver.find_best_action(st, observation=obs)


def build_checked_solver(method: str, seed: int, ruleset: str, extra: dict):
    """Build + smoke-probe. Leaf-backed methods degrade gracefully when
    the build lacks leaf-ort (or the model file is bad): retry with the
    deterministic handcrafted evaluator and report the substitution."""
    extra = dict(extra)
    solver = build_solver(method, seed, **extra)
    leaf_dependent = method.strip().lower().startswith(("leaf", "resolving"))
    try:
        probe_solver(solver, ruleset)
        return solver, None, extra
    except Exception:
        if not leaf_dependent:
            raise
        # leaf ONNX unusable (no leaf-ort build or bad file) — degrade to
        # the deterministic handcrafted evaluator, never silently crash
        extra["leaf_model"] = (
            None
            if method.strip().lower() in ("leaf_policy", "leafpolicy", "leaf")
            else "builtin:handcrafted"
        )
        solver = build_solver(method, seed, **extra)
        probe_solver(solver, ruleset)  # a failure here is a real error — propagate
        return solver, "leaf model unavailable - using handcrafted leaf evaluator", extra


TIER_METHOD = ["heuristic", "ismcts", "pimc", "leaf_policy", "resolving"]

# Method/archetype names the engine accepts — validated at form level so a
# typo shows an error instead of silently falling back to heuristic.
KNOWN_METHODS = {
    "random",
    "heuristic",
    "ismcts",
    "pimc",
    "endgame",
    "resolving",
    "resolving_adaptive",
    "leaf_policy",
    "archetype",
}
KNOWN_ARCHETYPES = {
    "heuristic",
    "materialist",
    "yaku_chaser",
    "banker",
    "gambler",
    "timid",
    "random",
}


# ── Widgets ───────────────────────────────────────────────────────────────


class CardTile(Static):
    """One hanafuda card: short month + face, border coloured by rank."""

    def __init__(self, card: _engine.Card | None = None) -> None:
        super().__init__()
        self.card = card
        if card is None:  # face-down opponent card
            self.update("░▒░\n░▒░")
            self.add_class("back")
        else:
            _, name, rank = card_label(card)
            self.update(f"{MONTHS_SHORT[card.month]}\n{name or '·'}")
            self.add_class(rank)


class WrappedRow(Vertical):
    """A card strip that wraps onto extra rows instead of overflowing.

    `per_row` comes from the terminal width, so cards never slide under
    other widgets — narrow terminals simply get more rows.
    """

    def set_cards(self, cards: list, per_row: int) -> None:
        self.remove_children()
        for i in range(0, len(cards), per_row):
            row = HorizontalGroup()
            self.mount(row)
            for c in cards[i : i + per_row]:
                row.mount(CardTile(c))


class ActionList(OptionList):
    """The legal-action picker — the only door into `apply_action`."""

    def set_actions(self, actions: list[_engine.Action]) -> None:
        self.clear_options()
        for a in actions:
            self.add_option(Option(action_rich(a), id=str(a.action_key)))
        self.highlighted = 0


# ── Screens ───────────────────────────────────────────────────────────────


class SetupScreen(Screen):
    """Difficulty / ruleset / seed, plus an advanced solver panel."""

    CSS = """
    .spacer { height: 1fr; }
    #titlebar { height: auto; align-horizontal: center; }
    #title { width: auto; text-align: center; text-style: bold; color: gold;
             border: heavy gold; padding: 0 3; }
    #subtitle { width: 100%; text-align: center; color: #888; }
    #hint { width: 100%; text-align: center; color: #777; }
    .h3 { color: #aaa; padding: 0 1; }
    #form { height: auto; align-horizontal: center; }
    #form > Vertical { width: 38; height: 12; padding: 0 1;
                       border: round #333; margin: 0 1; }
    #form Input { height: 1; border: none; padding: 0 1;
                  background: $boost; }
    #tier { height: 1fr; }
    #ruleset { height: auto; }
    #advbar { height: auto; align-horizontal: center; }
    #adv { width: auto; }
    #advanced { height: auto; align-horizontal: center; }
    #advanced Label { text-align: center; }
    #advanced Input { width: 56; }
    #startbar { height: auto; align-horizontal: center; }
    #start { width: 32; }
    """

    def compose(self) -> ComposeResult:
        yield Vertical(classes="spacer")
        with Horizontal(id="titlebar"):
            yield Label("H A N A F U D A\n    K O I - K O I", id="title")
        yield Label(
            "a koi-maestro terminal game — arrows pick · Enter confirms · Tab/click",
            id="hint",
        )
        with Horizontal(id="form"):
            with Vertical():
                yield Label("Opponent", classes="h3")
                yield OptionList(
                    *[Option(f"{n}  —  {d}", id=str(i)) for i, (n, d) in enumerate(TIERS)],
                    id="tier",
                )
            with Vertical():
                yield Label("Ruleset", classes="h3")
                yield OptionList(
                    Option("nintendo", id="nintendo"),
                    Option("fuda_wiki", id="fuda_wiki"),
                    id="ruleset",
                )
                yield Label("Seed (empty = random)", classes="h3")
                yield Input(placeholder="e.g. 42", id="seed")
                yield Label("Rounds (0 = open ended)", classes="h3")
                yield Input(value="12", id="rounds")
        with Horizontal(id="advbar"):
            yield Checkbox("Advanced solver settings", id="adv")
        yield Vertical(id="advanced")
        with Horizontal(id="startbar"):
            yield Button("Start match", variant="success", id="start")
        yield Vertical(classes="spacer")
        yield Footer()

    def on_mount(self) -> None:
        tier = self.query_one("#tier", OptionList)
        tier.highlighted = 0
        self.query_one("#ruleset", OptionList).highlighted = 0
        tier.focus()

    # Enter/click on a list confirms the highlight and moves focus to the
    # next control, so the screen walks like a short wizard.
    def on_option_list_option_selected(self, ev: OptionList.OptionSelected) -> None:
        if ev.option_list.id == "tier":
            self.query_one("#ruleset", OptionList).focus()
        elif ev.option_list.id == "ruleset":
            self.query_one("#seed", Input).focus()

    def on_input_submitted(self, ev: Input.Submitted) -> None:
        if ev.input.id == "seed":
            self.query_one("#rounds", Input).focus()
        elif ev.input.id == "rounds":
            self.query_one("#start", Button).focus()
        elif ev.input.id == "m_method":
            self.query_one("#m_extra", Input).focus()
        elif ev.input.id == "m_extra":
            self.query_one("#start", Button).focus()

    def on_checkbox_changed(self, event: Checkbox.Changed) -> None:
        panel = self.query_one("#advanced", Vertical)
        panel.remove_children()
        if event.value:
            panel.mount(
                Label(
                    "Method: random | heuristic | archetype:<name> | ismcts | "
                    "pimc | endgame | leaf_policy | resolving",
                    classes="h3",
                ),
                Input(placeholder="method (default: tier choice)", id="m_method"),
                Input(
                    placeholder="extra settings, e.g. max_simulations=512 adaptive=true",
                    id="m_extra",
                ),
            )
            # opening the panel can push Start below the fold — after the
            # new children settle, scroll just enough to keep it visible
            self.set_timer(
                0.05,
                lambda: self.query_one("#start", Button).scroll_visible(animate=False),
            )

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id != "start":
            return
        cfg = self._collect()
        if cfg is not None:
            self.app.push_screen(GameScreen(cfg))

    def _collect(self) -> dict | None:
        """Validate the form; return a config dict or None (error shown)."""
        err = self.query_one("#title", Label)
        try:
            tier_el = self.query_one("#tier", OptionList)
            tier = int(tier_el.highlighted or 0)
            ruleset_el = self.query_one("#ruleset", OptionList)
            ruleset = ruleset_el.highlighted
            ruleset = ["nintendo", "fuda_wiki"][ruleset or 0]
            seed_txt = self.query_one("#seed", Input).value.strip()
            seed = int(seed_txt) if seed_txt else random.randrange(1 << 64)
            if not 0 <= seed < (1 << 64):
                raise ValueError("seed must fit u64")
            rounds = max(0, int(self.query_one("#rounds", Input).value or "12"))
            method, extra = TIER_METHOD[tier], {}
            if self.query_one("#adv", Checkbox).value:
                raw = self.query_one("#m_method", Input).value.strip()
                if raw:
                    if raw.startswith("archetype:"):
                        method, extra["archetype"] = "archetype", raw.split(":", 1)[1]
                    else:
                        method = raw
                for pair in self.query_one("#m_extra", Input).value.split():
                    k, v = pair.split("=", 1)
                    extra[k] = (
                        {"true": True, "false": False}.get(v)
                        if v in ("true", "false")
                        else (int(v) if v.lstrip("-").isdigit() else v)
                    )
            if method == "archetype" and extra.get("archetype") not in KNOWN_ARCHETYPES:
                raise ValueError("archetype needs a known name")
            # eager build + smoke move — Start only launches a solver
            # that actually works (bad method/keys/values/model all
            # surface here); leaf fallbacks come back as a note
            _, note, extra = build_checked_solver(method, seed, ruleset, extra)
        except (ValueError, IndexError, OverflowError, NoMatches, TypeError, RuntimeError) as e:
            err.update(f"[red]Invalid settings — {e}")
            return None
        return {
            "seed": seed,
            "ruleset": ruleset,
            "rounds": rounds,
            "method": method,
            "extra": extra,
            "note": note,
        }


class RoundEndModal(ModalScreen[str]):
    """Round result — offers the next round or ending the match."""

    CSS = """
    RoundEndModal { align: center middle; }
    #modal { width: 64; height: auto; border: round gold; padding: 1 2;
             background: $surface; }
    #modal_text { padding-bottom: 1; }
    #modal Button { margin: 0 1; }
    """

    def __init__(self, text: str, match_over: bool) -> None:
        super().__init__()
        self._text, self._over = text, match_over

    def compose(self) -> ComposeResult:
        with Vertical(id="modal"):
            yield Label(self._text, id="modal_text")
            if not self._over:
                yield Button("Next round [Enter]", id="next", variant="primary")
            yield Button("Finish match", id="finish")
            yield Button("Back to setup", id="setup")

    def on_button_pressed(self, event: Button.Pressed) -> None:
        self.dismiss(event.button.id)

    BINDINGS: ClassVar = [
        Binding("enter", "next", show=False),
        Binding("escape", "cancel", show=False),
    ]

    def action_next(self) -> None:
        self.dismiss("finish" if self._over else "next")

    def action_cancel(self) -> None:
        self.dismiss(None)  # -> back to setup


class GameScreen(Screen):
    """The board. Layout: header bar, opponent strip, field, hand, sidebar."""

    BINDINGS: ClassVar = [
        Binding("q", "quit_app", "Quit"),
        Binding("n", "new_match", "New match"),
        Binding("s", "to_setup", "Setup"),
    ]

    CSS = """
    #status { dock: top; text-align: center; text-style: bold; color: gold;
              background: $boost; padding: 0 1; }
    .zone { color: #888; text-align: center; }
    #drawn { text-align: center; color: cyan; }
    #opphand, #hand, #field { height: auto; }
    WrappedRow { height: auto; align-horizontal: center; }
    WrappedRow HorizontalGroup { height: auto; }
    CardTile { width: 11; height: 4; margin: 0 1 0 0; border: round #555;
               content-align: center middle; }
    CardTile.hikari { border: round gold; }
    CardTile.tane { border: round green; }
    CardTile.tanzaku { border: round red; }
    CardTile.tanzaku-blue { border: round blue; }
    CardTile.kasu { border: round #444; color: #999; }
    CardTile.back { border: round #333; color: #555; }
    #bottom { dock: bottom; height: auto; border-top: solid #444;
              padding: 0 1; }
    #bl { width: 30; height: auto; }
    #bl LoadingIndicator { height: 1; }
    #actions { width: 1fr; height: 10; padding-left: 1; }
    #prompt { text-style: bold; padding-top: 1; }
    #keyhint { color: #777; }
    #main { height: 1fr; }
    """

    def __init__(self, cfg: dict) -> None:
        super().__init__()
        self.cfg = cfg
        self.you = _engine.Player.South  # human is always South
        self.opp_seat = _engine.Player.North
        self.state: _engine.GameState | None = None
        self.obs: _engine.PublicObservation | None = None
        self.opp: _engine.Solver | None = None
        self.aux: _engine.Solver | None = None  # empty-ledger stand-in
        self.round_seed = cfg["seed"]
        self.thinking = False
        self.gen = 0  # round generation — kills stale workers

    # ── lifecycle ────────────────────────────────────────────────────
    def compose(self) -> ComposeResult:
        yield Static("KOI-KOI — koi-maestro", id="status")
        with VerticalScroll(id="main"):
            yield Static("OPPONENT", classes="zone")
            yield WrappedRow(id="opphand")
            yield Static("FIELD", classes="zone")
            yield WrappedRow(id="field")
            yield Static("DRAWN: —", id="drawn")
            yield Static("YOUR HAND", classes="zone")
            yield WrappedRow(id="hand")
        with Horizontal(id="bottom"):
            with Vertical(id="bl"):
                yield Static("", id="score")
                yield Rule()
                yield Static("YOUR MOVE", id="prompt")
                yield Static("arrows pick · Enter plays", id="keyhint")
                yield LoadingIndicator(id="think")
            yield ActionList(id="actions")
        yield Footer()

    def on_mount(self) -> None:
        self.query_one("#think").display = False
        try:
            self.opp = build_solver(self.cfg["method"], seed=self.cfg["seed"], **self.cfg["extra"])
        except Exception as exc:  # noqa: BLE001                       # bad advanced settings
            self.app.notify(f"Solver build failed: {exc} — using heuristic", severity="error")
            self.opp = build_solver("heuristic", seed=self.cfg["seed"])
        if self.cfg.get("note"):
            self.app.notify(self.cfg["note"], severity="warning")
        self.new_round()

    # ── round / leg bookkeeping ─────────────────────────────────────
    def new_round(self) -> None:
        """Deal (or continue) one round and wire the resolver's ledger."""
        self.gen += 1  # any in-flight solver answer is stale now
        self.thinking = False
        self.state = (
            _engine.GameState.deal(self.round_seed, self.cfg["ruleset"])
            if self.state is None
            else self.state.next_round(self.round_seed)
        )
        self.obs = _engine.PublicObservation(self.state.public_view(self.opp_seat))
        self.refresh_board()
        self.advance()

    def apply(self, action: _engine.Action) -> None:
        """Apply + feed the public ledger (resolver needs every action)."""
        drawn = self.state.drawn if action.kind == "resolve_stock" else None
        actor = self.state.active
        self.state.apply_action(action)
        self.obs.push(_engine.LedgerEntry(actor, action, drawn))
        self.refresh_board()

    def advance(self) -> None:
        """Drive the state machine: opponent moves, or ask the human."""
        if self.state.is_ended:
            self.end_round()
        elif self.state.active == self.opp_seat:
            self.opponent_move()
        else:
            self.prompt_human()

    def prompt_human(self) -> None:
        actions = self.state.legal_actions()
        phase = self.state.phase
        label = {
            "hand": "Choose a card to play",
            "stock": "Resolve the drawn card",
            "stop": "Koi-Koi or Shobu?",
        }.get(phase, "Your move")
        if phase == "stop":
            base = self.state.public_view(self.you).base_score
            if base is not None:
                label = f"Yaku worth {base} — call Koi-Koi or take the points?"
        self.query_one("#prompt", Static).update(label)
        self.query_one("#actions", ActionList).set_actions(actions)
        self.query_one("#actions").focus()

    def on_option_list_option_selected(self, ev: OptionList.OptionSelected) -> None:
        if self.thinking or self.state is None or self.state.is_ended:
            return
        if self.state.active != self.you:
            return
        action = next(
            (a for a in self.state.legal_actions() if str(a.action_key) == ev.option.id), None
        )
        if action is None:
            return  # stale selection — ignore
        self.apply(action)
        self.advance()

    # ── opponent ─────────────────────────────────────────────────────
    def opponent_move(self) -> None:
        self.thinking = True
        self.query_one("#prompt", Static).update("Opponent thinking…")
        self.query_one("#think").display = True
        self._solver_step()

    @work(thread=True, exclusive=True)
    def _solver_step(self) -> None:
        gen = self.gen
        state, obs = self.state, self.obs  # read refs for the worker
        solver = self.opp
        # resolving refuses an empty ledger (a ply-0 North move) — a
        # light ismcts stand-in plays that single move instead of the
        # legal-first-action fallback
        if "resolving" in self.cfg["method"].lower() and obs is not None and len(obs) == 0:
            if self.aux is None:
                self.aux = build_solver("ismcts", seed=self.cfg["seed"] ^ 0x5150, iterations=500)
            solver = self.aux
        try:
            # resolving consumes the ledger; other backends ignore it —
            # always passing it keeps aliases like "resolving_adaptive" safe
            action = solver.find_best_action(state, observation=obs)
        except Exception as exc:  # noqa: BLE001
            self.app.call_from_thread(
                self.app.notify, f"Solver error ({exc}); falling back", severity="warning"
            )
            action = None
        self.app.call_from_thread(self._opp_done, action, gen)

    def _opp_done(self, action: _engine.Action | None, gen: int) -> None:
        if gen != self.gen or self.state is None:
            return  # stale answer from a discarded round
        self.thinking = False
        self.query_one("#think").display = False
        if self.state.is_ended:
            self.end_round()
            return
        if action is None:  # solver abstained or errored — still never stuck
            action = self.state.legal_actions()[0]
        self.apply(action)
        self.advance()

    # ── rendering ────────────────────────────────────────────────────
    def on_resize(self) -> None:
        self.refresh_board()  # re-wrap card rows for the new width

    def refresh_board(self) -> None:
        s = self.state
        if s is None:
            return
        # tile is 11 wide + 1 margin; leave a couple of columns slack
        per_row = max(4, (self.size.width - 2) // 12)
        hand = list(s.hand(self.you))
        opp_n = s.public_view(self.you).opponent_hand_count
        field = sorted(s.field, key=lambda c: (c.month, c.index_in_month))
        self.query_one("#hand", WrappedRow).set_cards(hand, per_row)
        self.query_one("#opphand", WrappedRow).set_cards([None] * opp_n, per_row)
        self.query_one("#field", WrappedRow).set_cards(field, per_row)
        # Action panel: grow to a fixed 14 option rows when the terminal
        # affords it (the spare rows just breathe), shrink first on small
        # screens — the table always keeps its space. Any leftover slack
        # pools between the hand and the bar.
        tile_rows = lambda n: -(-n // per_row) * 4  # tiles are 4 rows tall
        table_h = 4 + tile_rows(opp_n) + tile_rows(len(field)) + tile_rows(len(hand))
        avail = self.size.height - table_h - 3  # status bar + footer + border
        self.query_one("#actions").styles.height = max(6, min(14, avail))
        drawn = s.drawn
        if drawn is not None:
            m, n, _ = card_label(drawn)
            self.query_one("#drawn", Static).update(f"DRAWN: {m} {n}")
        else:
            self.query_one("#drawn", Static).update("DRAWN: —")
        sc = s.score
        kk = s.koi_koi_calls
        caps = s.public_view(self.you).captured  # always (south, north)
        rnd = s.round if s.is_ended else s.round + 1
        t = Text()
        t.append(f"ROUND {rnd} · stock {s.stock_count}", style="bold #d8d8d8")
        t.append(f"\ndealer: {seat_of(s.dealer)}", style="#9a9a9a")
        t.append("\n" + "─" * 24, style="#3a3a3a")
        t.append(f"\n{'':<5}{'score':>6}{'koi':>5}{'capt':>6}", style="#777777")
        t.append(f"\n{'you':<5}{sc[0]:>6}{kk[0]:>5}{len(caps[0]):>6}", style="#e8e8e8")
        t.append(f"\n{'opp':<5}{sc[1]:>6}{kk[1]:>5}{len(caps[1]):>6}", style="#8a8a8a")
        self.query_one("#score", Static).update(t)

    # ── round / match end ────────────────────────────────────────────
    def end_round(self) -> None:
        s = self.state
        margin = s.leg_margin(self.you)
        last = s.last_yaku_score
        text = (
            f"Round over — margin {'+' if margin >= 0 else ''}{margin} "
            f"for you\n(yaku score  you {last[0]} — {last[1]} opp)\n"
            f"Match score: you {s.score[0]} — {s.score[1]} opp"
        )
        # `round` is incremented at settlement, so on an ended state it is
        # the count of completed rounds — the match ends when it reaches
        # the configured length.
        over = self.cfg["rounds"] > 0 and s.round >= self.cfg["rounds"]
        if over:
            win = s.score[0] - s.score[1]
            text += f"\n\nMATCH OVER — you {'won' if win > 0 else 'lost' if win < 0 else 'drew'} {win:+d}"
        self.app.push_screen(RoundEndModal(text, over), self._round_choice)

    def _round_choice(self, choice: str | None) -> None:
        if choice == "next":
            self.round_seed = random.randrange(1 << 63)
            self.new_round()
        elif choice == "finish":
            self.app.pop_screen()  # back to setup
        else:
            self.action_to_setup()

    # ── app-level actions ────────────────────────────────────────────
    def action_new_match(self) -> None:
        # fresh deal + fresh solver seed — a "new match" is a new game,
        # not a deterministic replay of the old one
        self.round_seed = random.randrange(1 << 64)
        try:
            self.opp = build_solver(
                self.cfg["method"], seed=random.randrange(1 << 64), **self.cfg["extra"]
            )
        except Exception as exc:  # noqa: BLE001
            self.app.notify(f"Solver rebuild failed ({exc}); keeping current")
        self.state = None
        self.new_round()

    def action_to_setup(self) -> None:
        self.app.pop_screen()

    def action_quit_app(self) -> None:
        self.app.exit()


def seat_of(player: _engine.Player) -> str:
    return "you" if player == _engine.Player.South else "opp"


class KoiKoiApp(App):
    """Terminal Koi-Koi — engine-driven, deadlock-free by construction."""

    TITLE = "Koi-Koi"
    SUB_TITLE = "koi-maestro"

    def on_mount(self) -> None:
        self.push_screen(SetupScreen())


if __name__ == "__main__":
    KoiKoiApp().run()
