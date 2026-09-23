//! Canonical rulesets and variant configuration for Koi-Koi.

/// A named rule variant.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ruleset {
    /// Nintendo's published Koi-Koi rules: one Koi-Koi call per round,
    /// opponent-Koi-Koi double, 7+ Yaku double, null round is a draw.
    #[default]
    Nintendo,
    /// Fuda Wiki / common Western interpretation: multiple Koi-Koi calls
    /// may be allowed depending on house agreement.
    FudaWiki,
    /// A configurable house ruleset.
    House(HouseRules),
}

/// Fine-grained rule options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HouseRules {
    /// How many times a player may call Koi-Koi in a single round.
    pub koi_koi_calls_per_round: u8,
    /// Whether the opponent's previous Koi-Koi call doubles the round score.
    pub opponent_koi_koi_doubles: bool,
    /// Whether a Yaku total of 7 or more doubles the round score.
    pub seven_plus_doubles: bool,
    /// How a null round (no Yaku) is resolved.
    pub null_round: NullRoundResolution,
    /// Whether an opponent may call Koi-Koi after the other player has already
    /// called it (sometimes called a "take-back"). Nintendo rules forbid this;
    /// Fuda Wiki / house rules may allow it.
    pub take_back_koi_koi: bool,
    /// Fixed payment awarded to a lucky hand (Teshi or FourPairs). Zero
    /// disables lucky-hand wins — such deals are void and redealt instead.
    /// Six points is the common convention for an unearned deal win.
    pub anomaly_win_points: u32,
    /// Yaku point values and per-category scoring policy.
    pub yaku_points: YakuPoints,
}

/// Point values and per-category yaku policy for a ruleset.
///
/// All yaku categories are best-per-category by default; set
/// `best_yaku_per_category_only` to `false` to allow stacking within a category
/// where the variant permits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct YakuPoints {
    pub gokou: u32,
    pub shikou: u32,
    pub ame_shikou: u32,
    pub sankou: u32,
    pub inoshikacho: u32,
    pub tane_threshold: u32,
    pub tane_base: u32,
    pub tane_extra: u32,
    pub akatan: u32,
    pub aotan: u32,
    pub akatan_aotan: u32,
    pub tanzaku_threshold: u32,
    pub tanzaku_base: u32,
    pub tanzaku_extra: u32,
    pub hanami: u32,
    pub tsukimi: u32,
    pub all_viewing: u32,
    pub kasu_threshold: u32,
    pub kasu_base: u32,
    pub kasu_extra: u32,
    pub sake_cup_counts_as_kasu: bool,
    pub sake_cup_double_kasu: bool,
    pub best_yaku_per_category_only: bool,
}

impl YakuPoints {
    pub const fn nintendo() -> Self {
        Self {
            gokou: 10,
            shikou: 8,
            ame_shikou: 7,
            sankou: 5,
            inoshikacho: 5,
            tane_threshold: 5,
            tane_base: 1,
            tane_extra: 1,
            akatan: 5,
            aotan: 5,
            akatan_aotan: 10,
            tanzaku_threshold: 5,
            tanzaku_base: 1,
            tanzaku_extra: 1,
            hanami: 5,
            tsukimi: 5,
            all_viewing: 0,
            kasu_threshold: 10,
            kasu_base: 1,
            kasu_extra: 1,
            sake_cup_counts_as_kasu: true,
            sake_cup_double_kasu: false,
            best_yaku_per_category_only: true,
        }
    }

    pub const fn fuda_wiki() -> Self {
        Self {
            gokou: 15,
            shikou: 8,
            ame_shikou: 7,
            sankou: 6,
            inoshikacho: 5,
            tane_threshold: 5,
            tane_base: 1,
            tane_extra: 1,
            akatan: 5,
            aotan: 5,
            akatan_aotan: 10,
            tanzaku_threshold: 5,
            tanzaku_base: 1,
            tanzaku_extra: 1,
            hanami: 5,
            tsukimi: 5,
            all_viewing: 0,
            kasu_threshold: 10,
            kasu_base: 1,
            kasu_extra: 1,
            sake_cup_counts_as_kasu: true,
            sake_cup_double_kasu: false,
            best_yaku_per_category_only: true,
        }
    }
}

/// How to resolve a round in which neither player forms a Yaku.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NullRoundResolution {
    /// The dealer keeps the deal and the round is worth 0 points.
    DealerKeepsDeal,
    /// The dealer keeps the deal and is awarded a small point.
    DealerWinsOnePoint,
    /// The round is a draw; the opponent becomes dealer.
    DrawOpponentDeals,
    /// The hand is re-dealt.
    Redeal,
}

impl Default for HouseRules {
    fn default() -> Self {
        Self::nintendo()
    }
}

impl Ruleset {
    /// Returns the canonical Nintendo rules.
    pub const fn nintendo() -> Self {
        Ruleset::Nintendo
    }

    pub const fn fuda_wiki() -> Self {
        Ruleset::FudaWiki
    }

    pub const fn house(rules: HouseRules) -> Self {
        Ruleset::House(rules)
    }

    pub const fn house_rules(&self) -> HouseRules {
        match self {
            Ruleset::Nintendo => HouseRules {
                koi_koi_calls_per_round: 1,
                opponent_koi_koi_doubles: true,
                seven_plus_doubles: true,
                null_round: NullRoundResolution::DrawOpponentDeals,
                take_back_koi_koi: false,
                anomaly_win_points: 6,
                yaku_points: YakuPoints::nintendo(),
            },
            Ruleset::FudaWiki => HouseRules {
                koi_koi_calls_per_round: 255,
                opponent_koi_koi_doubles: true,
                seven_plus_doubles: true,
                null_round: NullRoundResolution::DealerKeepsDeal,
                take_back_koi_koi: true,
                anomaly_win_points: 6,
                yaku_points: YakuPoints::fuda_wiki(),
            },
            Ruleset::House(rules) => *rules,
        }
    }

    /// Returns `true` if the opponent's Koi-Koi call doubles the score.
    pub fn opponent_koi_koi_doubles(&self) -> bool {
        self.house_rules().opponent_koi_koi_doubles
    }

    /// Returns `true` if a Yaku total of 7 or more doubles the score.
    pub fn seven_plus_doubles(&self) -> bool {
        self.house_rules().seven_plus_doubles
    }

    /// Returns the yaku point table for this ruleset.
    pub fn yaku_points(&self) -> YakuPoints {
        self.house_rules().yaku_points
    }

    /// Returns `true` if the Sake Cup also counts as a chaff card.
    pub fn sake_cup_counts_as_kasu(&self) -> bool {
        self.yaku_points().sake_cup_counts_as_kasu
    }

    /// Returns the null-round resolution rule.
    pub fn null_round_resolution(&self) -> NullRoundResolution {
        self.house_rules().null_round
    }

    /// Returns the maximum number of Koi-Koi calls allowed per round.
    pub fn koi_koi_calls_per_round(&self) -> u8 {
        self.house_rules().koi_koi_calls_per_round
    }

    /// Returns `true` if only the best Yaku per category is counted.
    pub fn best_yaku_per_category_only(&self) -> bool {
        self.yaku_points().best_yaku_per_category_only
    }

    /// Returns `true` if the opponent may call Koi-Koi after a Koi-Koi call.
    pub fn take_back_koi_koi(&self) -> bool {
        self.house_rules().take_back_koi_koi
    }

    /// Returns the fixed payment for a lucky hand (Teshi / FourPairs), or
    /// zero when the ruleset treats such deals as void redeals.
    pub fn anomaly_win_points(&self) -> u32 {
        self.house_rules().anomaly_win_points
    }

    /// Computes the final round score from the base Yaku total.
    ///
    /// `opponent_called_koi_koi` is `true` if the opponent has called Koi-Koi
    /// and `opponent_koi_koi_doubles` is enabled.
    pub fn apply_multipliers(&self, base: u32, opponent_called_koi_koi: bool) -> u32 {
        let mut score = base;
        if self.seven_plus_doubles() && base >= 7 {
            score *= 2;
        }
        if self.opponent_koi_koi_doubles() && opponent_called_koi_koi {
            score *= 2;
        }
        score
    }
}

impl HouseRules {
    pub const fn nintendo() -> Self {
        Ruleset::nintendo().house_rules()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nintendo_multipliers() {
        let rules = Ruleset::nintendo();
        assert!(rules.opponent_koi_koi_doubles());
        assert!(rules.seven_plus_doubles());
        assert_eq!(rules.apply_multipliers(5, false), 5);
        assert_eq!(rules.apply_multipliers(7, false), 14);
        assert_eq!(rules.apply_multipliers(5, true), 10);
        assert_eq!(rules.apply_multipliers(7, true), 28);
    }

    #[test]
    fn house_rules_override_nintendo_defaults() {
        let mut rules = HouseRules::nintendo();
        rules.koi_koi_calls_per_round = 3;
        rules.opponent_koi_koi_doubles = false;
        let ruleset = Ruleset::House(rules);

        assert_eq!(ruleset.koi_koi_calls_per_round(), 3);
        assert!(!ruleset.opponent_koi_koi_doubles());
        assert_eq!(ruleset.apply_multipliers(7, true), 14); // no opponent double
    }
}
