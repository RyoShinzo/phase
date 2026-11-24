//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `quantity.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::ability::{
    AggregateFunction, ControllerRef, QuantityRef, TargetFilter, TypedFilter,
};

fn fixed_legs(values: &[i32]) -> Vec<QuantityExpr> {
    values
        .iter()
        .map(|&value| QuantityExpr::Fixed { value })
        .collect()
}

#[test]
fn number_disjunction_serial_stops_at_clause_comma() {
    // Sarkhan's Unsealing (the text after "with power ").
    assert_eq!(
        parse_number_disjunction("4, 5, or 6, this enchantment deals 4 damage to any target."),
        Ok((
            ", this enchantment deals 4 damage to any target.",
            fixed_legs(&[4, 5, 6])
        ))
    );
    // Kozilek's Unsealing (the text after "with mana value ").
    assert_eq!(
        parse_number_disjunction(
            "4, 5, or 6, create two 0/1 colorless eldrazi spawn creature tokens"
        ),
        Ok((
            ", create two 0/1 colorless eldrazi spawn creature tokens",
            fixed_legs(&[4, 5, 6])
        ))
    );
    // A single number before a clause comma is not a list (Firemind's
    // Foresight: "an instant card with mana value 3, reveal it, ...").
    assert!(parse_number_disjunction("3, reveal it, and put it into your hand.").is_err());
    // An "or" comparator closes no number leg (Bitter Work; As Foretold).
    assert!(parse_number_disjunction("4 or greater, draw a card").is_err());
    assert!(
        parse_number_disjunction("x or less, where x is the number of time counters on ~").is_err()
    );
    // "base" is not a number leg (Vhati il-Dal).
    assert!(parse_number_disjunction("1 or base toughness 1").is_err());
    // The article guard: "a" never reads as the leg 1, before a space or
    // before punctuation (the closing "or a," would otherwise make the
    // clause commas list-internal, reading [4, 5, 1]).
    assert!(parse_number_disjunction("a creature spell with power 4, 5, or 6").is_err());
    assert!(parse_number_disjunction("4, 5, or a, draw a card").is_err());
}

#[test]
fn number_disjunction_two_legs() {
    // Transit Mage.
    assert_eq!(
        parse_number_disjunction("4 or 5, reveal it, put it into your hand, then shuffle."),
        Ok((
            ", reveal it, put it into your hand, then shuffle.",
            fixed_legs(&[4, 5])
        ))
    );
    // Frodo, Determined Hero.
    assert_eq!(
        parse_number_disjunction("2 or 3 to ~."),
        Ok((" to ~.", fixed_legs(&[2, 3])))
    );
}

/// CR 120.1 + CR 120.9: Notorious Throng's "the damage dealt to your
/// opponents this turn" is every source's damage, combat and noncombat,
/// to the opponents.
#[test]
fn test_parse_damage_dealt_to_your_opponents_this_turn() {
    let (rest, q) =
        parse_damage_dealt_this_turn_ref("the damage dealt to your opponents this turn").unwrap();
    assert_eq!(rest, "");
    assert_eq!(
        q,
        QuantityRef::DamageDealtThisTurn {
            source: Box::new(TargetFilter::Any),
            target: Box::new(TargetFilter::And {
                filters: vec![
                    TargetFilter::Player,
                    TargetFilter::Typed(TypedFilter::default().controller(ControllerRef::Opponent),),
                ],
            }),
            aggregate: AggregateFunction::Sum,
            group_by: None,
            damage_kind: DamageKindFilter::Any,

            channel: DamageChannel::Total,
        }
    );
}
