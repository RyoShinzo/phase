//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `oracle_casting.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::ability::{FilterProp, TargetFilter};
use crate::types::mana::ManaCost;

/// CR 118.9 + CR 601.2f: Sephara, Sky's Blade's alternative cost keeps its
/// mana part as a `Mana` leg beside the tap leg, so the engine can make
/// {W} the base that replaces the mana cost and tap the four creatures
/// with it.
#[test]
fn alt_cost_mana_and_tap_creatures_composite() {
    use crate::types::mana::ManaCostShard;
    let option = parse_spell_casting_option_line(
        "You may pay {W} and tap four untapped creatures you control with flying rather than pay this spell's mana cost.",
        "Sephara, Sky's Blade",
    )
    .expect("alt-cost should parse");
    let SpellCastingOption {
        kind: crate::types::ability::SpellCastingOptionKind::AlternativeCost,
        cost: Some(AbilityCost::Composite { costs }),
        condition: None,
    } = option
    else {
        panic!("expected Composite alt-cost, got {option:?}");
    };
    assert_eq!(costs.len(), 2, "{costs:?}");
    assert_eq!(
        costs[0],
        AbilityCost::Mana {
            cost: ManaCost::Cost {
                generic: 0,
                shards: vec![ManaCostShard::White],
            },
        }
    );
    let AbilityCost::TapCreatures {
        ref requirement,
        filter: TargetFilter::Typed(ref typed),
        direction: crate::types::ability::TapStateChange::Tap,
    } = costs[1]
    else {
        panic!("expected TapCreatures, got {:?}", costs[1]);
    };
    assert_eq!(requirement.fixed_count(), Some(4));
    assert!(
        typed
            .properties
            .iter()
            .any(|p| matches!(p, FilterProp::WithKeyword { .. })),
        "the tapped creatures need flying: {typed:?}"
    );
}
