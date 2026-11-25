//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `oracle_cost.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them.

use super::*;
use crate::types::ability::{ControllerRef, TapStateChange};
use crate::types::mana::ManaColor;

/// The single `TapCreatures` leg of `cost`, alone or inside a `Composite`.
fn tap_creatures_leg(cost: &AbilityCost) -> (u32, &TargetFilter, TapStateChange) {
    match cost {
        AbilityCost::TapCreatures {
            requirement,
            filter,
            direction,
        } => (
            requirement
                .fixed_count()
                .expect("a counted tap or untap cost"),
            filter,
            *direction,
        ),
        AbilityCost::Composite { costs } => {
            let mut legs = costs
                .iter()
                .filter(|cost| matches!(cost, AbilityCost::TapCreatures { .. }));
            let leg = legs.next().expect("a TapCreatures leg");
            assert!(legs.next().is_none(), "one TapCreatures leg in {costs:?}");
            tap_creatures_leg(leg)
        }
        other => panic!("expected a TapCreatures cost, got {other:?}"),
    }
}

/// The filter names a creature the payer controls and carries neither
/// "tapped" nor "untapped": the direction is the authority for that.
fn assert_creatures_you_control(filter: &TargetFilter) {
    let TargetFilter::Typed(typed) = filter else {
        panic!("expected a typed creature filter, got {filter:?}");
    };
    assert_eq!(typed.controller, Some(ControllerRef::You), "{typed:?}");
    assert!(
        !typed
            .properties
            .iter()
            .any(|prop| matches!(prop, FilterProp::Tapped | FilterProp::Untapped)),
        "the tapped status is the direction's, not the filter's: {typed:?}"
    );
}

/// CR 701.26b + CR 601.2b: "untap N tapped [creatures] you control" is the
/// untap direction of the counted tap-creatures cost, for any count word: a,
/// two, fifteen (Halo Fountain's three abilities).
#[test]
fn untap_n_tapped_creatures_you_control_is_an_untap_cost() {
    for (text, count) in [
        ("{W}, {T}, Untap a tapped creature you control", 1),
        ("{W}{W}, {T}, Untap two tapped creatures you control", 2),
        (
            "{W}{W}{W}{W}{W}, {T}, Untap fifteen tapped creatures you control",
            15,
        ),
    ] {
        let cost = parse_oracle_cost(text);
        let AbilityCost::Composite { costs } = &cost else {
            panic!("{text:?}: expected a Composite cost, got {cost:?}");
        };
        assert!(
            costs.iter().any(|leg| matches!(leg, AbilityCost::Tap)),
            "{text:?}: the {{T}} leg stays: {costs:?}"
        );
        let (parsed_count, filter, direction) = tap_creatures_leg(&cost);
        assert_eq!(parsed_count, count, "{text:?}");
        assert_eq!(direction, TapStateChange::Untap, "{text:?}");
        assert_creatures_you_control(filter);
    }
}

/// CR 701.26b: the filter keeps the qualities between the count and the noun
/// ("two tapped blue creatures", Crackleburr).
#[test]
fn untap_cost_keeps_the_filter_qualities() {
    let cost = parse_oracle_cost("Untap two tapped blue creatures you control");
    let (count, filter, direction) = tap_creatures_leg(&cost);
    assert_eq!((count, direction), (2, TapStateChange::Untap));
    assert_creatures_you_control(filter);
    let TargetFilter::Typed(typed) = filter else {
        unreachable!("checked above");
    };
    assert!(
        typed.properties.iter().any(|prop| matches!(
            prop,
            FilterProp::HasColor {
                color: ManaColor::Blue
            }
        )),
        "{typed:?}"
    );
}

/// CR 701.26a: the tap forms keep the tap direction and parse as before.
#[test]
fn tap_n_untapped_creatures_keeps_the_tap_direction() {
    for (text, count) in [
        ("Tap an untapped creature you control", 1),
        ("Tap two untapped creatures you control", 2),
        ("Tapped four untapped Humans you control", 4),
    ] {
        let (parsed_count, _, direction) = tap_creatures_leg(&parse_oracle_cost(text));
        assert_eq!(
            (parsed_count, direction),
            (count, TapStateChange::Tap),
            "{text:?}"
        );
    }
}

/// CR 107.6: "{Q}" and an untap of the source itself are no untap-creatures
/// cost.
#[test]
fn untap_symbol_stays_the_source_untap() {
    assert_eq!(parse_oracle_cost("{Q}"), AbilityCost::Untap);
}
