//! The opponent model: Bayesian reweighting of the resolver's belief from
//! the observed public ledger, over a portfolio of scripted archetypes.
//!
//! Koi-Koi makes the consistent-past computation *deterministic* where
//! the sibling codebase needed a backward un-deal: stock draws never enter hands, so the
//! opponent's hand at a past turn is exactly `current_hand ∪ cards they
//! played at or after that turn` — every other input to their decision
//! (field, piles, drawn card, scores) is public. Per hypothesized world the
//! model replays the ledger with that world's hand substituted and asks how
//! likely each archetype would have been to make the observed decisions.
//!
//! The posterior is `prior(world) × P(ledger | world)` shrunk toward the
//! uniform belief: `w = (1−λ)·uniform + λ·model` with `λ = n/(n+K)` capped
//! strictly below one — the Dirichlet-pseudocount bound from Ganzfried-
//! style Bayesian exploitation. With no observed opponent decisions `λ` is
//! zero and the belief is untouched; even a fully contradicted world keeps
//! `(1−λ)` of the uniform floor, so the model can never drive a world to
//! zero mass. The shrunk archetype posterior can additionally gate
//! exploitation: when its top mass falls below the gate the model's signal
//! is too weak to act on.

use koi_core::yaku::{rank, score_yaku, CardRank};
use koi_core::{Action, CaptureChoice, Card, CardSet, KoiGameState, Player, TurnPhase};

use super::belief::World;
use super::reconstruct::LedgerReplay;

/// A scripted opponent policy — deterministic except `Random`, whose
/// likelihood is uniform by definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Archetype {
    /// The P1 greedy yaku-aware heuristic.
    Heuristic,
    /// Immediate card material only, no yaku planning; banks a decent score.
    Materialist,
    /// Maximizes own yaku-score deltas; always continues when offered.
    YakuChaser,
    /// Banks every stop offer; heuristic play otherwise.
    Banker,
    /// Continues every stop offer; materialist play otherwise.
    Gambler,
    /// Banks every stop offer; materialist play otherwise.
    Timid,
    /// Uniform over legal actions.
    Random,
}

impl Archetype {
    /// Every archetype in portfolio order.
    pub const ALL: [Self; 7] = [
        Self::Heuristic,
        Self::Materialist,
        Self::YakuChaser,
        Self::Banker,
        Self::Gambler,
        Self::Timid,
        Self::Random,
    ];

    /// The stable config/manifest name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::Materialist => "materialist",
            Self::YakuChaser => "yaku_chaser",
            Self::Banker => "banker",
            Self::Gambler => "gambler",
            Self::Timid => "timid",
            Self::Random => "random",
        }
    }

    /// The deterministic pick, or `None` when the archetype is
    /// intrinsically non-deterministic (`Random` — uniform likelihood).
    pub fn pick(self, state: &KoiGameState) -> Option<Action> {
        match self {
            Self::Heuristic => crate::heuristic::find_best_action(state),
            Self::Materialist => Some(materialist_pick(state)),
            Self::YakuChaser => Some(yaku_chaser_pick(state)),
            Self::Banker => match state.phase {
                TurnPhase::AwaitingStopDecision { .. } => Some(Action::Shobu),
                _ => crate::heuristic::find_best_action(state),
            },
            Self::Gambler => match state.phase {
                TurnPhase::AwaitingStopDecision { .. } => Some(if state.legal_actions().contains(&Action::KoiKoi) {
                    Action::KoiKoi
                } else {
                    Action::Shobu
                }),
                _ => Some(materialist_pick(state)),
            },
            Self::Timid => match state.phase {
                TurnPhase::AwaitingStopDecision { .. } => Some(Action::Shobu),
                _ => Some(materialist_pick(state)),
            },
            Self::Random => None,
        }
    }
}

impl std::str::FromStr for Archetype {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "heuristic" => Ok(Self::Heuristic),
            "materialist" => Ok(Self::Materialist),
            "yaku_chaser" | "yakuchaser" => Ok(Self::YakuChaser),
            "banker" => Ok(Self::Banker),
            "gambler" => Ok(Self::Gambler),
            "timid" => Ok(Self::Timid),
            "random" => Ok(Self::Random),
            other => Err(format!("unknown archetype '{other}'")),
        }
    }
}

fn card_material(card: Card) -> i64 {
    match rank(card) {
        CardRank::Hikari => 20,
        CardRank::SakeCup => 14,
        CardRank::Tane => 10,
        CardRank::Tanzaku(_) => 6,
        CardRank::Kasu => 1,
    }
}

fn matched_field(state: &KoiGameState, month: koi_core::Month, capture: CaptureChoice) -> CardSet {
    match capture {
        CaptureChoice::NoMatch => CardSet::EMPTY,
        CaptureChoice::Single(card) | CaptureChoice::Pair(card) => CardSet::from_card(card),
        CaptureChoice::Triple => state.field.month_cards(month),
    }
}

fn capture_material(state: &KoiGameState, played: Card, capture: CaptureChoice) -> i64 {
    let taken = matched_field(state, played.month(), capture);
    let mut value: i64 = taken.into_iter().map(card_material).sum::<i64>() + card_material(played);
    if capture == CaptureChoice::NoMatch {
        // The card lands on the field — captured value is zero; slightly
        // prefer depositing low-value cards.
        value = -card_material(played);
    }
    value
}

fn materialist_pick(state: &KoiGameState) -> Action {
    match state.phase {
        TurnPhase::AwaitingStopDecision { base_score } => {
            if base_score >= 3 && state.legal_actions().contains(&Action::Shobu) {
                Action::Shobu
            } else if state.legal_actions().contains(&Action::KoiKoi) {
                Action::KoiKoi
            } else {
                Action::Shobu
            }
        }
        _ => state
            .legal_actions()
            .into_iter()
            .map(|action| {
                let value = match action {
                    Action::PlayFromHand { card, capture } => capture_material(state, card, capture),
                    Action::ResolveStock { capture } => match state.phase {
                        TurnPhase::AwaitingStockResolution { drawn } => capture_material(state, drawn, capture),
                        _ => i64::MIN / 2,
                    },
                    _ => i64::MIN / 2,
                };
                (value, action.action_key(), action)
            })
            .max_by(|(v1, k1, _), (v2, k2, _)| v1.cmp(v2).then(k2.cmp(k1)))
            .map(|(_, _, action)| action)
            .unwrap_or(Action::Shobu),
    }
}

fn yaku_chaser_pick(state: &KoiGameState) -> Action {
    match state.phase {
        TurnPhase::AwaitingStopDecision { .. } => {
            if state.legal_actions().contains(&Action::KoiKoi) {
                Action::KoiKoi
            } else {
                Action::Shobu
            }
        }
        _ => {
            let me = state.active.index();
            let mine = state.captured[me];
            let base = score_yaku(mine, &state.rules) as i64;
            state
                .legal_actions()
                .into_iter()
                .map(|action| {
                    let gained = match action {
                        Action::PlayFromHand { card, capture } => {
                            matched_field(state, card.month(), capture).insert(card)
                        }
                        Action::ResolveStock { capture } => match state.phase {
                            TurnPhase::AwaitingStockResolution { drawn } => {
                                matched_field(state, drawn.month(), capture).insert(drawn)
                            }
                            _ => CardSet::EMPTY,
                        },
                        _ => CardSet::EMPTY,
                    };
                    let value = if gained.is_empty() {
                        i64::MIN / 4
                    } else {
                        score_yaku(mine.union(gained), &state.rules) as i64 - base
                    };
                    (value, action.action_key(), action)
                })
                .max_by(|(v1, k1, _), (v2, k2, _)| v1.cmp(v2).then(k2.cmp(k1)))
                .map(|(_, _, action)| action)
                .unwrap_or(Action::Shobu)
        }
    }
}

/// The opponent model: a portfolio prior, a tremble mass, and the
/// shrinkage/gate schedule.
#[derive(Debug, Clone)]
pub struct OpponentModel {
    /// Prior over archetypes; must sum to one.
    pub portfolio: Vec<(Archetype, f64)>,
    /// Tremble mass ε smoothing each deterministic archetype's likelihood.
    pub epsilon: f64,
    /// Dirichlet pseudocount `K` in `λ = n/(n+K)`.
    pub shrink_k: f64,
    /// Hard cap on λ; strictly below one so the uniform floor survives.
    pub lambda_max: f64,
    /// Activation gate: when set, OX exploitation engages only where the
    /// shrunk archetype posterior's top mass reaches the threshold.
    pub gate: Option<f64>,
}

impl Default for OpponentModel {
    fn default() -> Self {
        Self {
            portfolio: vec![
                (Archetype::Heuristic, 0.30),
                (Archetype::Materialist, 0.16),
                (Archetype::YakuChaser, 0.16),
                (Archetype::Banker, 0.12),
                (Archetype::Gambler, 0.12),
                (Archetype::Timid, 0.07),
                (Archetype::Random, 0.07),
            ],
            epsilon: 0.05,
            shrink_k: 8.0,
            lambda_max: 0.85,
            gate: Some(0.55),
        }
    }
}

impl OpponentModel {
    /// The shrinkage coefficient for `n` observed opponent decisions.
    pub fn lambda(&self, observed_opponent_turns: usize) -> f64 {
        let n = observed_opponent_turns as f64;
        (n / (n + self.shrink_k)).min(self.lambda_max)
    }

    /// Reweights `worlds` in place by `posterior ∝ prior × likelihood`
    /// under the archetype mixture, shrunk toward uniform by `λ(n)`.
    /// Returns without mutating when the ledger has no opponent decisions.
    pub fn reweight(&self, worlds: &mut [World], replay: &LedgerReplay, observer: Player) {
        let n = opponent_frame_count(replay, observer);
        if n == 0 || worlds.is_empty() {
            return;
        }
        let lambda = self.lambda(n);
        let joints = worlds
            .iter()
            .map(|world| self.world_joints(world, replay, observer))
            .collect::<Vec<_>>();

        // Per-world marginal likelihood under the archetype mixture,
        // computed in the product domain — pure IEEE `*`, identical on
        // every target (ln/exp route to platform libm and can differ in
        // the last ulp; P2-D4's correctly-rounded-primitives contract).
        let marginals: Vec<f64> = joints
            .iter()
            .map(|joint| marginal_likelihood(joint, &self.portfolio))
            .collect();
        if marginals.iter().all(|m| *m <= 0.0) {
            // Every world contradicts the ledger under every archetype: the
            // model has no signal — keep the chance prior.
            return;
        }
        let uniform = 1.0 / worlds.len() as f64;
        let mut mass = 0.0;
        let mut posterior: Vec<f64> = worlds
            .iter()
            .zip(&marginals)
            .map(|(world, &marginal)| {
                let m = world.weight.max(0.0) * marginal;
                mass += m;
                m
            })
            .collect();
        if mass <= 0.0 {
            return;
        }
        for (world, m) in worlds.iter_mut().zip(&mut posterior) {
            let model = *m / mass;
            world.weight = (1.0 - lambda) * uniform + lambda * model;
        }
    }

    /// The effective archetype posterior: `P(a | ledger) ∝ Σ_w w·π_a·L_a^w`
    /// marginalized over the belief, shrunk toward the prior by `λ(n)`.
    /// `None` when the belief is empty; the prior itself when no opponent
    /// decisions were observed.
    pub fn archetype_posterior(
        &self,
        worlds: &[World],
        replay: &LedgerReplay,
        observer: Player,
    ) -> Option<Vec<(Archetype, f64)>> {
        if worlds.is_empty() {
            return None;
        }
        let n = opponent_frame_count(replay, observer);
        if n == 0 {
            return Some(self.portfolio.clone());
        }
        // Posterior_a = Σ_w w_w · P(a | w, ledger), where the per-world
        // archetype posterior is `prior_a·joint_a / Σ prior·joint` —
        // product domain, correctly-rounded primitives only.
        let mut mass = vec![0.0_f64; self.portfolio.len()];
        let mut total = 0.0;
        for world in worlds {
            let joints = self.world_joints(world, replay, observer);
            let z: f64 = joints
                .iter()
                .enumerate()
                .map(|(i, &j)| j * self.portfolio[i].1.max(1e-12))
                .sum();
            if z <= 0.0 {
                continue;
            }
            let w = world.weight.max(0.0);
            total += w;
            for (slot, &joint) in joints.iter().enumerate() {
                mass[slot] += w * joint * self.portfolio[slot].1.max(1e-12) / z;
            }
        }
        if total <= 0.0 {
            return None;
        }
        let lambda = self.lambda(n);
        let norm: f64 = mass.iter().sum();
        if norm <= 0.0 {
            return None;
        }
        Some(
            self.portfolio
                .iter()
                .enumerate()
                .map(|(slot, (archetype, prior))| (*archetype, (1.0 - lambda) * prior + lambda * (mass[slot] / norm)))
                .collect(),
        )
    }

    /// Whether the shrunk archetype posterior clears the activation gate —
    /// the OX arm's firing condition. Always true when no gate is set.
    pub fn gate_passes(&self, posterior: &[(Archetype, f64)]) -> bool {
        match self.gate {
            Some(threshold) => posterior.iter().map(|(_, p)| *p).fold(0.0, f64::max) >= threshold,
            None => true,
        }
    }

    /// Per-archetype likelihoods of the observed opponent decisions in
    /// one world: `joints[a] = Π_t P_a(action_t | state_t^w)`, kept in the
    /// product domain — pure IEEE multiplication, bit-identical on every
    /// target (`ln`/`exp` route to platform libm and can differ in the
    /// last ulp; P2-D4's correctly-rounded-primitives contract). Factors
    /// are ≥ ε/legal_count ≥ 0.006 over ≤ ~32 opponent frames, so products
    /// stay ≥ ~1e-71 — far above f64 underflow. A world that cannot
    /// produce the observed past returns all zeros (zero likelihood).
    fn world_joints(&self, world: &World, replay: &LedgerReplay, observer: Player) -> Vec<f64> {
        let opponent = observer.opponent();
        let mut joints = vec![1.0_f64; self.portfolio.len()];
        for (index, frame) in replay.frames.iter().enumerate() {
            if frame.player != opponent {
                continue;
            }
            let Some(state) = world_state_at_frame(world, replay, index, opponent) else {
                // The world cannot produce the observed past — zero
                // likelihood under every archetype.
                return vec![0.0; self.portfolio.len()];
            };
            let legal = state.legal_actions();
            let legal_count = legal.len().max(1) as f64;
            for (slot, (archetype, _)) in self.portfolio.iter().enumerate() {
                let p = match archetype {
                    Archetype::Random => 1.0 / legal_count,
                    other => {
                        let pick = other.pick(&state);
                        let hit = pick == Some(frame.action);
                        (1.0 - self.epsilon) * f64::from(u8::from(hit)) + self.epsilon / legal_count
                    }
                };
                joints[slot] *= p;
            }
        }
        joints
    }
}

/// The number of observed opponent decisions in the replay.
fn opponent_frame_count(replay: &LedgerReplay, observer: Player) -> usize {
    let opponent = observer.opponent();
    replay.frames.iter().filter(|frame| frame.player == opponent).count()
}

/// `Σ_a prior_a · joints_a` — the mixture marginal in the product domain.
/// Zero when every archetype contradicts the ledger in this world.
fn marginal_likelihood(joints: &[f64], portfolio: &[(Archetype, f64)]) -> f64 {
    joints
        .iter()
        .enumerate()
        .map(|(i, &j)| j * portfolio[i].1.max(1e-12))
        .sum()
}

/// The world-consistent state at a ledger frame: the frame's public zones
/// with the opponent's hand substituted for the world hypothesis. The
/// opponent's hand at `index` is `world.hand ∪ {opp plays at ≥ index}` —
/// deterministic because draws never enter hands (P3-D4).
fn world_state_at_frame(world: &World, replay: &LedgerReplay, index: usize, opponent: Player) -> Option<KoiGameState> {
    let frame = &replay.frames[index];
    let mut hand = world.opponent_hand;
    for later in &replay.frames[index..] {
        if later.player == opponent {
            if let Action::PlayFromHand { card, .. } = later.action {
                hand = hand.insert(card);
            }
        }
    }
    // The frame's hidden pool: everything not publicly located.
    let public = frame.state.hands[opponent.opponent().index()]
        .union(frame.state.field)
        .union(frame.state.captured[0])
        .union(frame.state.captured[1]);
    let unseen = CardSet::ALL.difference(public);
    if !hand.is_subset(unseen) {
        return None;
    }
    let mut stock: Vec<Card> = Vec::new();
    if let TurnPhase::AwaitingStockResolution { drawn } = frame.state.phase {
        if !unseen.contains(drawn) || hand.contains(drawn) {
            return None;
        }
        stock.push(drawn);
    }
    let head = stock.first().copied();
    stock.extend(
        unseen
            .into_iter()
            .filter(|card| !hand.contains(*card) && Some(*card) != head),
    );
    frame.state.rebuild_hidden(opponent, hand, &stock).ok()
}

#[cfg(test)]
mod tests {
    use koi_core::{deal_from_seed, LedgerEntry, PublicObservation, Ruleset};
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    use super::*;
    use crate::resolving::belief::build_belief;
    use crate::resolving::reconstruct::replay_ledger;

    /// Plays a scripted leg: the opponent (non-dealer) always Shobus when
    /// offered and otherwise plays the first legal action — the ledger
    /// discriminates `Banker` from `Gambler` on the first stop offer.
    fn scripted_leg(seed: u64) -> (KoiGameState, PublicObservation, Player) {
        let (mut state, anomaly) = KoiGameState::new_deal(deal_from_seed(seed), Ruleset::nintendo());
        assert!(anomaly.is_none());
        let observer = state.dealer;
        let opponent = observer.opponent();
        let initial = state.public_view(observer);
        let mut entries = Vec::new();
        while !state.is_ended() {
            let action = if state.active == opponent {
                match state.phase {
                    TurnPhase::AwaitingStopDecision { .. } => Action::Shobu,
                    _ => state.legal_actions()[0],
                }
            } else {
                state.legal_actions()[0]
            };
            let drawn = match state.phase {
                TurnPhase::AwaitingStockResolution { drawn } => Some(drawn),
                _ => None,
            };
            entries.push(LedgerEntry {
                player: state.active,
                action,
                drawn,
            });
            state = state.apply_action(action).unwrap();
        }
        (state, PublicObservation { initial, entries }, observer)
    }

    /// A mid-leg observation so the model has opponent turns to score.
    fn mid_leg(seed: u64, cutoff: usize) -> (KoiGameState, LedgerReplay, Player) {
        let (_end, observation, observer) = scripted_leg(seed);
        let truncated = PublicObservation {
            initial: observation.initial.clone(),
            entries: observation.entries[..cutoff.min(observation.entries.len())].to_vec(),
        };
        let replay = replay_ledger(&truncated).unwrap();
        (replay.state, replay, observer)
    }

    #[test]
    fn reweight_leaves_the_prior_when_no_opponent_turns() {
        let (state, replay, observer) = mid_leg(5, 0);
        let mut worlds = build_belief(&state, observer, CardSet::ALL, 16, &mut SmallRng::seed_from_u64(1)).unwrap();
        let before: Vec<f64> = worlds.iter().map(|w| w.weight).collect();
        OpponentModel::default().reweight(&mut worlds, &replay, observer);
        let after: Vec<f64> = worlds.iter().map(|w| w.weight).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn reweight_shifts_mass_but_keeps_the_uniform_floor() {
        let (state, replay, observer) = mid_leg(9, 14);
        let model = OpponentModel::default();
        let mut worlds = build_belief(&state, observer, CardSet::ALL, 24, &mut SmallRng::seed_from_u64(2)).unwrap();
        model.reweight(&mut worlds, &replay, observer);
        let lambda = model.lambda(super::opponent_frame_count(&replay, observer));
        let floor = (1.0 - lambda) / worlds.len() as f64;
        let total: f64 = worlds.iter().map(|w| w.weight).sum();
        assert!((total - 1.0).abs() < 1e-6, "weights renormalize to one: {total}");
        for world in &worlds {
            assert!(
                world.weight >= floor - 1e-12,
                "uniform floor survives: {} < {floor}",
                world.weight
            );
        }
    }

    #[test]
    fn posterior_recovers_the_scripted_archetype() {
        // The scripted opponent always Shobus — after a stop offer the
        // posterior must favor Banker/Timid over Gambler.
        let (state, replay, observer) = mid_leg(13, 20);
        let model = OpponentModel::default();
        let worlds = build_belief(&state, observer, CardSet::ALL, 24, &mut SmallRng::seed_from_u64(3)).unwrap();
        if opponent_frame_count(&replay, observer) == 0 {
            return; // leg had no opponent decisions to score
        }
        let posterior = model.archetype_posterior(&worlds, &replay, observer).unwrap();
        let mass = |a: Archetype| posterior.iter().find(|(x, _)| *x == a).map(|(_, p)| *p).unwrap_or(0.0);
        let total: f64 = posterior.iter().map(|(_, p)| p).sum();
        assert!((total - 1.0).abs() < 1e-9);
        // Banker/Timid both Shobu always — together they should dominate
        // Gambler (who always continues) after any observed Shobu.
        if replay
            .frames
            .iter()
            .any(|f| f.player == observer.opponent() && f.action == Action::Shobu)
        {
            assert!(
                mass(Archetype::Banker) + mass(Archetype::Timid) > mass(Archetype::Gambler),
                "posterior must favor Shobu archetypes: {posterior:?}"
            );
        }
    }

    #[test]
    fn gate_clears_only_on_a_concentrated_posterior() {
        let model = OpponentModel::default();
        // After enough observed decisions the shrinkage λ approaches
        // lambda_max — the posterior can clear the gate only when the
        // raw mass concentrates on one archetype.
        let concentrated: Vec<(Archetype, f64)> = vec![
            (Archetype::Heuristic, 0.30),
            (Archetype::Materialist, 0.16),
            (Archetype::YakuChaser, 0.16),
            (Archetype::Banker, 0.12),
            (Archetype::Gambler, 0.12),
            (Archetype::Timid, 0.07),
            (Archetype::Random, 0.07),
        ];
        assert!(
            !model.gate_passes(&concentrated),
            "uniform prior must not clear the gate"
        );

        // Simulated clear-archetype posterior: ~80% on Gambler.
        let clear: Vec<(Archetype, f64)> = vec![
            (Archetype::Heuristic, 0.05),
            (Archetype::Materialist, 0.03),
            (Archetype::YakuChaser, 0.03),
            (Archetype::Banker, 0.02),
            (Archetype::Gambler, 0.80),
            (Archetype::Timid, 0.03),
            (Archetype::Random, 0.04),
        ];
        assert!(model.gate_passes(&clear), "concentrated posterior must clear the gate");

        // The gate is conservative by design: it requires ~59% raw mass
        // after full shrinkage — a deliberate safety margin, not a bug.
        let borderline: Vec<(Archetype, f64)> = vec![
            (Archetype::Heuristic, 0.54),
            (Archetype::Materialist, 0.10),
            (Archetype::YakuChaser, 0.10),
            (Archetype::Banker, 0.08),
            (Archetype::Gambler, 0.08),
            (Archetype::Timid, 0.05),
            (Archetype::Random, 0.05),
        ];
        assert!(
            !model.gate_passes(&borderline),
            "borderline posterior must not clear the gate"
        );
    }

    #[test]
    fn archetype_picks_are_legal() {
        let (state, anomaly) = KoiGameState::new_deal(deal_from_seed(21), Ruleset::nintendo());
        assert!(anomaly.is_none());
        let legal = state.legal_actions();
        for archetype in Archetype::ALL {
            if let Some(action) = archetype.pick(&state) {
                assert!(legal.contains(&action), "{archetype:?} picked an illegal action");
            }
        }
    }
}
