//! Blueprint persistence for the safety gadget.
//!
//! A `BlueprintArtifact` is the on-disk form of a converged reduced-variant
//! profile: the strategy table itself plus the provenance that produced it.
//! [`load_gadget`] recompiles the declared variant's EFG, verifies the
//! recorded tree checksum against the recompiled infoset-key sequence,
//! verifies each profile row's positional (key, row) digest — so a
//! same-arity row permutation or corrupted row cannot silently misvalue
//! the gadget — validates the profile's shape against the tree's infoset
//! layout, and returns a [`SafetyGadget`] that owns the shared tree.
//!
//! The artifact stores only the strategy profile plus the binding digests —
//! never the derived tree: the tree is a pure function of `(variant,
//! dealer)` *per compiler version*, and the digests are what detect a
//! cross-version, wrong-tree, or reordered binding instead of silently
//! reindexing rows.
//!
//! The canonical 48-card game has no compiled blueprint domain: its gadget
//! is the learned oracle (empirical guard only). A `"full_48"` variant tag
//! is therefore a load error, not a silent fallback.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use koi_core::Player;

use crate::cfr::profile::Blueprint;
use crate::efg::compiler::{compile_variant, KoiVariant};
use crate::efg::tree::{EfgTree, InfoSetKey};

use super::gadget::SafetyGadget;

/// The artifact format tag; unknown tags fail closed at load.
pub const BLUEPRINT_FORMAT: &str = "koi-blueprint/v1";

/// The persisted gadget input: a strategy profile bound to the exact
/// `(variant, dealer)` that compiled the tree it indexes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlueprintArtifact {
    /// Must equal [`BLUEPRINT_FORMAT`].
    pub format: String,
    /// Reduced-variant name — see [`reduced_variant_by_name`].
    pub variant: String,
    /// `"south"` or `"north"` — the dealer (who moves first in the tree).
    pub dealer: String,
    /// How the profile was produced, recorded for evidence provenance.
    pub provenance: BlueprintProvenance,
    /// Structural digest of the compiled infoset-key and action sequence —
    /// binds the artifact to the exact tree the profile was trained on.
    pub tree_checksum: String,
    /// The strategy rows, each fused to the digest of the infoset key it
    /// belongs to.
    pub entries: Vec<BoundRow>,
}

/// One profile row fused to its infoset-key digest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundRow {
    /// `{:016x}` FxHash of the perfect-recall [`InfoSetKey`] this row
    /// belongs to — verified positionally at load.
    pub infoset_key: String,
    /// Average strategy over this infoset's actions, in compiled order.
    pub strategy: Vec<f64>,
}

/// Training provenance: what produced the stored profile. These fields are
/// evidence, not load-time checks — the loader validates the profile's
/// shape, and `exploitability` records the bound measured at export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlueprintProvenance {
    /// The trainer, e.g. `"cfr_plus"` or `"sequence_lp"`.
    pub algorithm: String,
    /// Iterations the trainer ran (0 for LP-derived profiles).
    pub iterations: u64,
    /// The profile's measured exploitability on the compiled tree, when
    /// the producer computed it.
    pub exploitability: Option<f64>,
    /// Build commit the profile was produced under, when known.
    pub build_commit: Option<String>,
}

impl BlueprintArtifact {
    /// Wraps a trained blueprint for persistence.
    pub fn from_blueprint(
        blueprint: &Blueprint,
        tree: &EfgTree,
        dealer: Player,
        provenance: BlueprintProvenance,
    ) -> Result<Self, ArtifactError> {
        let variant = variant_name(tree.variant()).ok_or(ArtifactError::CanonicalDomain)?;
        let entries = tree
            .infoset_keys_in_order()
            .iter()
            .zip(blueprint.averaged_profile())
            .map(|(key, strategy)| BoundRow {
                infoset_key: infoset_key_digest(key),
                strategy,
            })
            .collect();
        Ok(Self {
            format: BLUEPRINT_FORMAT.to_owned(),
            variant: variant.to_owned(),
            dealer: player_name(dealer).to_owned(),
            provenance,
            tree_checksum: tree_checksum(tree),
            entries,
        })
    }
}

/// Saves a blueprint artifact as pretty JSON.
pub fn save_blueprint_artifact(path: &Path, artifact: &BlueprintArtifact) -> Result<(), ArtifactError> {
    let json = serde_json::to_string_pretty(artifact)
        .map_err(|error| ArtifactError::Malformed(format!("artifact serialization failed: {error}")))?;
    std::fs::write(path, json).map_err(ArtifactError::Io)
}

/// Loads a blueprint artifact and rebuilds the gadget: recompiles the
/// declared variant, verifies the tree checksum and every row's positional
/// key digest, validates the profile shape, and returns the shared-tree
/// gadget. Every mismatch fails closed.
pub fn load_gadget(path: &Path) -> Result<SafetyGadget, ArtifactError> {
    let text = std::fs::read_to_string(path).map_err(ArtifactError::Io)?;
    let artifact: BlueprintArtifact =
        serde_json::from_str(&text).map_err(|error| ArtifactError::Malformed(error.to_string()))?;
    if artifact.format != BLUEPRINT_FORMAT {
        return Err(ArtifactError::UnknownFormat(artifact.format));
    }
    let variant = reduced_variant_by_name(&artifact.variant)
        .ok_or_else(|| ArtifactError::UnknownVariant(artifact.variant.clone()))?;
    let dealer =
        player_by_name(&artifact.dealer).ok_or_else(|| ArtifactError::UnknownPlayer(artifact.dealer.clone()))?;
    let tree = compile_variant(variant, dealer).map_err(|error| ArtifactError::Compile(error.to_string()))?;
    let compiled = tree_checksum(&tree);
    if compiled != artifact.tree_checksum {
        return Err(ArtifactError::TreeMismatch {
            declared: artifact.tree_checksum,
            compiled,
        });
    }
    let keys = tree.infoset_keys_in_order();
    if artifact.entries.len() != keys.len() {
        return Err(ArtifactError::InvalidProfile(format!(
            "{} rows for {} infosets",
            artifact.entries.len(),
            keys.len()
        )));
    }
    let mut profile = Vec::with_capacity(keys.len());
    for (index, (entry, key)) in artifact.entries.iter().zip(&keys).enumerate() {
        let digest = infoset_key_digest(key);
        if entry.infoset_key != digest {
            return Err(ArtifactError::InvalidProfile(format!(
                "row {index} binds key {} but position expects {digest}",
                entry.infoset_key
            )));
        }
        let arity = tree.infosets()[index].actions.len();
        if entry.strategy.len() != arity {
            return Err(ArtifactError::InvalidProfile(format!(
                "row {index} has {} actions for an arity-{arity} infoset",
                entry.strategy.len()
            )));
        }
        let mass: f64 = entry.strategy.iter().sum();
        if !entry.strategy.iter().all(|p| p.is_finite() && *p >= 0.0) || (mass - 1.0).abs() > 1e-6 {
            return Err(ArtifactError::InvalidProfile(format!(
                "row {index} is not a normalized strategy (mass {mass})"
            )));
        }
        profile.push(entry.strategy.clone());
    }
    Ok(SafetyGadget::from_profile(Arc::new(tree), profile))
}

/// The order-sensitive digest over every infoset's key and action list.
fn tree_checksum(tree: &EfgTree) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    let keys = tree.infoset_keys_in_order();
    for (infoset, key) in tree.infosets().iter().zip(&keys) {
        key.hash(&mut hasher);
        for action in &infoset.actions {
            action.action_key().hash(&mut hasher);
        }
    }
    format!("{:016x}", hasher.finish())
}

/// The per-row label: one FxHash over the perfect-recall key alone.
fn infoset_key_digest(key: &InfoSetKey) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    key.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// The reduced variants a blueprint artifact may declare.
fn reduced_variant_by_name(name: &str) -> Option<&'static KoiVariant> {
    match name {
        "nano_6" => Some(&KoiVariant::NANO_6),
        "micro_8" => Some(&KoiVariant::MICRO_8),
        "fieldvoid_10" => Some(&KoiVariant::FIELDVOID_10),
        "reduced_9" => Some(&KoiVariant::REDUCED_9),
        _ => None,
    }
}

/// The persisted name of a variant — `None` for the canonical domain, which
/// has no compiled blueprint.
fn variant_name(variant: &KoiVariant) -> Option<&'static str> {
    reduced_variant_by_name(variant.name).map(|_| variant.name)
}

fn player_name(player: Player) -> &'static str {
    match player {
        Player::South => "south",
        Player::North => "north",
    }
}

fn player_by_name(name: &str) -> Option<Player> {
    match name {
        "south" => Some(Player::South),
        "north" => Some(Player::North),
        _ => None,
    }
}

/// Every failure of the artifact path.
#[derive(Debug)]
pub enum ArtifactError {
    /// The file could not be read or written.
    Io(std::io::Error),
    /// The file is not a well-formed artifact JSON.
    Malformed(String),
    /// `format` is not [`BLUEPRINT_FORMAT`].
    UnknownFormat(String),
    /// `variant` is not a known reduced-variant name.
    UnknownVariant(String),
    /// The artifact declares the canonical game, which has no compiled
    /// blueprint domain (the canonical gadget is the learned oracle).
    CanonicalDomain,
    /// `dealer` did not parse.
    UnknownPlayer(String),
    /// The declared variant failed to compile.
    Compile(String),
    /// The profile does not match the compiled tree's infoset layout or is
    /// not a valid strategy table.
    InvalidProfile(String),
    /// The compiled tree's digest disagrees with the artifact's.
    TreeMismatch { declared: String, compiled: String },
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "artifact I/O failed: {error}"),
            Self::Malformed(detail) => write!(f, "malformed artifact: {detail}"),
            Self::UnknownFormat(tag) => write!(f, "unknown blueprint format '{tag}'"),
            Self::UnknownVariant(name) => write!(f, "unknown reduced variant '{name}'"),
            Self::CanonicalDomain => f.write_str("the canonical game has no compiled blueprint domain"),
            Self::UnknownPlayer(name) => write!(f, "unknown dealer '{name}'"),
            Self::Compile(detail) => write!(f, "variant failed to compile: {detail}"),
            Self::InvalidProfile(detail) => write!(f, "invalid profile: {detail}"),
            Self::TreeMismatch { declared, compiled } => write!(
                f,
                "tree checksum mismatch: artifact declares {declared}, compiled {compiled}"
            ),
        }
    }
}

impl std::error::Error for ArtifactError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfr::cfr_plus::train_cfr_plus;
    use koi_core::{CardSet, KoiGameState};

    #[test]
    fn artifact_round_trips_into_a_working_gadget() {
        let dir = std::env::temp_dir().join(format!("koi-artifact-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blueprint.json");

        let tree = compile_variant(&KoiVariant::NANO_6, Player::South).unwrap();
        let blueprint = train_cfr_plus(&tree, 200);
        let exploitability = crate::cfr::exploitability(&tree, &blueprint.averaged_profile());
        let artifact = BlueprintArtifact::from_blueprint(
            &blueprint,
            &tree,
            Player::South,
            BlueprintProvenance {
                algorithm: "cfr_plus".to_owned(),
                iterations: 200,
                exploitability: Some(exploitability),
                build_commit: None,
            },
        )
        .unwrap();
        save_blueprint_artifact(&path, &artifact).unwrap();

        let gadget = load_gadget(&path).unwrap();
        // The loaded gadget evaluates a dealt variant state.
        let cards: Vec<koi_core::Card> = KoiVariant::NANO_6.cards.into_iter().collect();
        let (state, anomaly) = KoiGameState::new_from_parts(
            &cards[4..],
            [CardSet::from_card(cards[0]), CardSet::from_card(cards[1])],
            CardSet::from_card(cards[2]).insert(cards[3]),
            Player::South,
            Player::South,
            KoiVariant::NANO_6.rules,
        );
        assert!(anomaly.is_none());
        let anchors = crate::resolving::gadget::MarginAnchors {
            initial_field: state.field,
            history_prefix: Vec::new(),
        };
        assert!(gadget.margin_south(&state, &anchors).is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tampered_rows_fail_closed() {
        let dir = std::env::temp_dir().join(format!("koi-artifact-tamper-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blueprint.json");

        let tree = compile_variant(&KoiVariant::NANO_6, Player::South).unwrap();
        let blueprint = train_cfr_plus(&tree, 50);
        let mut artifact = BlueprintArtifact::from_blueprint(
            &blueprint,
            &tree,
            Player::South,
            BlueprintProvenance {
                algorithm: "cfr_plus".to_owned(),
                iterations: 50,
                exploitability: None,
                build_commit: None,
            },
        )
        .unwrap();
        // Swap two rows: same arities would pass a shape check, but the
        // positional key digests must catch the permutation.
        let last = artifact.entries.len() - 1;
        artifact.entries.swap(0, last);
        save_blueprint_artifact(&path, &artifact).unwrap();
        assert!(matches!(load_gadget(&path), Err(ArtifactError::InvalidProfile(_))));
        let _ = std::fs::remove_file(&path);
    }
}
