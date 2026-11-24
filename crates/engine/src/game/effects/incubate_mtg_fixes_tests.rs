//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `incubate.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::ability::{Effect, QuantityExpr, TargetFilter, TypeFilter, TypedFilter};
use crate::types::identifiers::ObjectId;
use crate::types::player::PlayerId;

fn make_incubate_ability(count: QuantityExpr) -> ResolvedAbility {
    ResolvedAbility::new(
        Effect::Incubate { count },
        vec![],
        ObjectId(100),
        PlayerId(0),
    )
}

/// CR 111.10i + CR 701.27a: an Incubator is a double-faced token, so a mass
/// transform turns each one to its Phyrexian face (The Argent Etchings: "Incubate 2
/// five times, then transform all Incubator tokens you control"). The mass
/// transform keeps only permanents `is_double_faced_permanent` knows, so with no
/// layout on the token's back face it transformed none of them.
#[test]
fn mass_transform_turns_every_incubator_token() {
    let mut state = GameState::new_two_player(42);
    let mut events = Vec::new();
    let incubate = make_incubate_ability(QuantityExpr::Fixed { value: 2 });
    for _ in 0..5 {
        resolve(&mut state, &incubate, &mut events).unwrap();
    }
    let incubators = TargetFilter::Typed(
        TypedFilter::new(TypeFilter::Artifact).subtype("Incubator".to_string()),
    );
    let transform = ResolvedAbility::new(
        Effect::Transform {
            target: incubators,
            scope: crate::types::ability::EffectScope::All,
        },
        vec![],
        ObjectId(100),
        PlayerId(0),
    );

    crate::game::effects::transform_effect::resolve(&mut state, &transform, &mut events).unwrap();

    let tokens: Vec<_> = state
        .battlefield
        .iter()
        .filter_map(|id| state.objects.get(id))
        .filter(|obj| obj.is_token)
        .collect();
    assert_eq!(tokens.len(), 5);
    for token in tokens {
        assert!(token.transformed, "every Incubator token must transform");
        assert_eq!(token.name, "Phyrexian Token");
        assert!(token.card_types.core_types.contains(&CoreType::Artifact));
        assert!(token.card_types.core_types.contains(&CoreType::Creature));
        assert_eq!(token.card_types.subtypes, vec!["Phyrexian".to_string()]);
        assert_eq!(
            token.counters.get(&CounterType::Plus1Plus1).copied(),
            Some(2)
        );
    }
}
