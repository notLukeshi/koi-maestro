//! Sequence-form extraction from a materialized EFG tree.
//!
//! The sequence form (von Stengel 1996; Koller, Megiddo & von Stengel 1996)
//! represents a two-player zero-sum extensive game in size linear in the
//! tree: one realization variable per player sequence, one treeplex equality
//! per information set, and a payoff matrix indexed by sequence pairs. It is
//! the exact endgame LP's input. Ported from a sibling research codebase's
//! `solver/efg/sequence.rs`; the walk is game-agnostic — only the player
//! names (South/North) and the dense-payoff cell cap (P2-D5) differ.

use std::collections::HashMap;

use koi_core::Player;

use super::tree::{EfgTree, NodeId, NodeType};

/// Dense payoff-matrix cell budget. Sequence-form LP solving targets
/// MICRO-scale domains; larger reduced trees use CFR + the O(nodes)
/// exploitability walk, which never materialize this matrix.
pub const MAX_PAYOFF_CELLS: usize = 64_000_000;

/// One player's sequence tree: the root sequence plus, per information set,
/// its parent sequence and one sequence per action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSequences {
    /// Includes the root sequence (id 0, the empty sequence).
    pub sequence_count: usize,
    /// Parallel to the player's information-set list.
    pub infosets: Vec<InfosetSequences>,
    /// The tree's global infoset id behind each local slot, so behavioral
    /// profiles keyed by tree infoset lift onto the sequence tree.
    pub global_infosets: Vec<usize>,
}

/// A sequence set's information-set record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfosetSequences {
    /// The player's sequence leading to this information set.
    pub parent: usize,
    /// One sequence id per action, in action order.
    pub actions: Vec<usize>,
}

impl PlayerSequences {
    fn new() -> Self {
        Self {
            sequence_count: 1,
            infosets: Vec::new(),
            global_infosets: Vec::new(),
        }
    }
}

/// The sequence-form data of a compiled zero-sum game.
#[derive(Debug, Clone)]
pub struct SequenceForm {
    pub south: PlayerSequences,
    pub north: PlayerSequences,
    /// Dense `|Σ_s| x |Σ_n|` row-major expected payoff for South: entry
    /// `(s, t)` accumulates `chance_reach * u_south` over terminals whose
    /// last South sequence is `s` and last North sequence is `t`.
    pub payoff: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceFormError {
    /// A player's information set was reached under two different parent
    /// sequences, which would violate perfect recall.
    ImperfectRecall,
    /// The dense payoff matrix exceeds `MAX_PAYOFF_CELLS`.
    TooLarge,
}

impl std::fmt::Display for SequenceFormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ImperfectRecall => f.write_str("an information set was reached under two different parent sequences"),
            Self::TooLarge => f.write_str("the dense payoff matrix exceeds MAX_PAYOFF_CELLS"),
        }
    }
}

impl std::error::Error for SequenceFormError {}

impl SequenceForm {
    pub fn south_sequence_count(&self) -> usize {
        self.south.sequence_count
    }

    pub fn north_sequence_count(&self) -> usize {
        self.north.sequence_count
    }

    pub fn payoff(&self, seq_south: usize, seq_north: usize) -> f64 {
        self.payoff[seq_south * self.north.sequence_count + seq_north]
    }

    /// The same game from North's perspective: the players' sequence trees
    /// swap and the payoff matrix negates and transposes.
    pub fn mirrored(&self) -> Self {
        let mut payoff = vec![0.0; self.north.sequence_count * self.south.sequence_count];
        for seq_s in 0..self.south.sequence_count {
            for seq_n in 0..self.north.sequence_count {
                payoff[seq_n * self.south.sequence_count + seq_s] =
                    -self.payoff[seq_s * self.north.sequence_count + seq_n];
            }
        }
        Self {
            south: self.north.clone(),
            north: self.south.clone(),
            payoff,
        }
    }

    /// Extracts the sequence form of a compiled tree.
    pub fn compile(tree: &EfgTree) -> Result<Self, SequenceFormError> {
        let mut extractor = Extractor {
            south: PlayerSequences::new(),
            north: PlayerSequences::new(),
            local_index: HashMap::new(),
            terminals: Vec::new(),
        };
        extractor.walk(tree, tree.root(), 1.0, 0, 0)?;

        let payoff_len = extractor
            .south
            .sequence_count
            .checked_mul(extractor.north.sequence_count)
            .filter(|&cells| cells <= MAX_PAYOFF_CELLS)
            .ok_or(SequenceFormError::TooLarge)?;
        let mut payoff = vec![0.0; payoff_len];
        for (seq_s, seq_n, value) in extractor.terminals {
            payoff[seq_s * extractor.north.sequence_count + seq_n] += value;
        }
        Ok(Self {
            south: extractor.south,
            north: extractor.north,
            payoff,
        })
    }
}

struct Extractor {
    south: PlayerSequences,
    north: PlayerSequences,
    /// Global tree infoset id -> per-player local index.
    local_index: HashMap<usize, usize>,
    terminals: Vec<(usize, usize, f64)>,
}

impl Extractor {
    fn walk(
        &mut self,
        tree: &EfgTree,
        id: NodeId,
        reach: f64,
        seq_s: usize,
        seq_n: usize,
    ) -> Result<(), SequenceFormError> {
        match tree.node(id) {
            NodeType::Terminal { utility_south } => {
                self.terminals.push((seq_s, seq_n, reach * utility_south));
            }
            NodeType::Chance { outcomes } => {
                for outcome in outcomes {
                    self.walk(tree, outcome.child, reach * outcome.probability, seq_s, seq_n)?;
                }
            }
            NodeType::Decision {
                player,
                infoset,
                actions,
                children,
            } => {
                let parent = match player {
                    Player::South => seq_s,
                    Player::North => seq_n,
                };
                let global_infoset = *infoset;
                let local = match self.local_index.get(&global_infoset) {
                    Some(&local) => local,
                    None => {
                        let local = match player {
                            Player::South => self.south.infosets.len(),
                            Player::North => self.north.infosets.len(),
                        };
                        let infoset_record = InfosetSequences {
                            parent,
                            actions: Vec::new(),
                        };
                        match player {
                            Player::South => {
                                self.south.infosets.push(infoset_record);
                                self.south.global_infosets.push(global_infoset);
                            }
                            Player::North => {
                                self.north.infosets.push(infoset_record);
                                self.north.global_infosets.push(global_infoset);
                            }
                        }
                        self.local_index.insert(global_infoset, local);
                        local
                    }
                };
                let action_sequences = {
                    let sequences = match player {
                        Player::South => &mut self.south,
                        Player::North => &mut self.north,
                    };
                    let record = &mut sequences.infosets[local];
                    if record.parent != parent {
                        return Err(SequenceFormError::ImperfectRecall);
                    }
                    if record.actions.is_empty() {
                        // Sequence IDs are per-infoset-action, not per-edge:
                        // every member of the infoset shares the same child
                        // sequences, so they are minted exactly once.
                        for _ in actions.iter().zip(children) {
                            record.actions.push(sequences.sequence_count);
                            sequences.sequence_count += 1;
                        }
                    }
                    record.actions.clone()
                };

                for (&child, sequence) in children.iter().zip(action_sequences) {
                    let (next_s, next_n) = match player {
                        Player::South => (sequence, seq_n),
                        Player::North => (seq_s, sequence),
                    };
                    self.walk(tree, child, reach, next_s, next_n)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use koi_core::Player;

    use super::super::compiler::{compile_variant, KoiVariant};
    use super::*;

    #[test]
    fn sequence_form_covers_every_compiled_infoset_and_terminal() {
        let tree = compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap();
        let form = SequenceForm::compile(&tree).unwrap();

        assert!(form.south.sequence_count > 1);
        assert!(form.north.sequence_count > 1);
        for sequences in [&form.south, &form.north] {
            for infoset in &sequences.infosets {
                assert!(infoset.parent < sequences.sequence_count);
                assert!(!infoset.actions.is_empty());
                assert!(infoset
                    .actions
                    .iter()
                    .all(|&sequence| sequence < sequences.sequence_count));
            }
        }

        let total: f64 = form.payoff.iter().map(|value| value.abs()).sum();
        assert!(total.is_finite() && total > 0.0);
    }

    #[test]
    fn sequence_form_matches_infoset_counts() {
        let tree = compile_variant(&KoiVariant::MICRO_8, Player::South).unwrap();
        let form = SequenceForm::compile(&tree).unwrap();
        let south_infosets = tree
            .infosets()
            .iter()
            .filter(|infoset| infoset.player == Player::South)
            .count();
        let north_infosets = tree
            .infosets()
            .iter()
            .filter(|infoset| infoset.player == Player::North)
            .count();
        assert_eq!(form.south.infosets.len(), south_infosets);
        assert_eq!(form.north.infosets.len(), north_infosets);
    }
}
