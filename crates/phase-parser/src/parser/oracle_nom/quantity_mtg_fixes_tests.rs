//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `quantity.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::ability::{
    AggregateFunction, ControllerRef, QuantityRef, TargetFilter, TypedFilter,
};

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
