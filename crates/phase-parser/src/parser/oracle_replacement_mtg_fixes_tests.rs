//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `oracle_replacement.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::ability::{ControllerRef, QuantityModification};

fn controlled_or_branch_types(valid_card: Option<TargetFilter>) -> Vec<Vec<TypeFilter>> {
    let Some(TargetFilter::Or { filters }) = valid_card else {
        panic!("expected controlled Or target filter");
    };
    filters
        .into_iter()
        .map(|filter| match filter {
            TargetFilter::Typed(TypedFilter {
                type_filters,
                controller: Some(ControllerRef::You),
                ..
            }) => type_filters,
            other => panic!("expected controlled typed branch, got {other:?}"),
        })
        .collect()
}

/// CR 303.4 + CR 614.1a: "to enchanted player" scopes the shield to the
/// player the Aura enchants, not the later "to that player" (any player).
#[test]
fn damage_curse_of_bloodletting_enchanted_player() {
    let def = parse_replacement_line(
        "If a source would deal damage to enchanted player, it deals double that damage to that player instead.",
        "Curse of Bloodletting",
    )
    .unwrap();
    assert_eq!(def.damage_modification, Some(DamageModification::Double));
    assert_eq!(
        def.damage_target_filter,
        Some(DamageTargetFilter::Player {
            player: DamageTargetPlayerScope::EnchantedPlayer,
        })
    );
}

#[test]
fn counter_doubling_replacement_actor_with_named_counter_type() {
    // CR 614.1a + CR 122.1: "If you would put one or more charge counters on an
    // artifact you control, put twice that many charge counters on that artifact
    // instead." The named type limits it to charge counters and the recipient to
    // artifacts you control (Lt. Cmdr. Geordi La Forge).
    let def = parse_replacement_line(
        "If you would put one or more charge counters on an artifact you control, put twice that many charge counters on that artifact instead.",
        "Lt. Cmdr. Geordi La Forge",
    )
    .unwrap();

    assert_eq!(def.event, ReplacementEvent::AddCounter);
    assert_eq!(
        def.quantity_modification,
        Some(QuantityModification::DOUBLE)
    );
    assert_eq!(
        def.counter_match,
        Some(CounterMatch::OfType(CounterType::Generic(
            "charge".to_string()
        )))
    );
    assert_eq!(def.valid_player, Some(ReplacementPlayerScope::You));
    assert!(matches!(
        def.valid_card,
        Some(TargetFilter::Typed(TypedFilter {
            ref type_filters,
            controller: Some(ControllerRef::You),
            ..
        })) if type_filters == &vec![TypeFilter::Artifact]
    ));
}

#[test]
fn counter_plus_one_replacement_laezel_permanents_or_yourself() {
    // CR 614.1a + CR 122.1: "If you would put one or more counters on a creature
    // or planeswalker you control or on yourself, put that many plus one ..."
    // (Lae'zel, Vlaakith's Champion). The recipient is the permanents named by
    // the type phrase or the controller: `valid_card` carries the permanents,
    // `counter_player_recipient` the player leg, and "you would put" scopes the
    // actor.
    let def = parse_replacement_line(
        "If you would put one or more counters on a creature or planeswalker you control or on yourself, put that many plus one of each of those kinds of counters on that permanent or player instead.",
        "Lae'zel, Vlaakith's Champion",
    )
    .unwrap();
    assert_eq!(def.event, ReplacementEvent::AddCounter);
    assert_eq!(
        def.quantity_modification,
        Some(QuantityModification::Plus { value: 1 })
    );
    assert_eq!(def.counter_match, None);
    assert_eq!(def.valid_player, Some(ReplacementPlayerScope::You));
    assert_eq!(
        def.counter_replacement_subject,
        CounterReplacementSubject::Actor
    );
    assert_eq!(
        def.counter_player_recipient,
        Some(ReplacementPlayerScope::You)
    );
    assert_eq!(
        controlled_or_branch_types(def.valid_card),
        vec![vec![TypeFilter::Creature], vec![TypeFilter::Planeswalker]]
    );
}

#[test]
fn counter_replacement_without_player_leg_reaches_no_player() {
    // CR 614.1a: a type phrase with no "or on yourself" leaves the recipient
    // object-only (Hardened Scales, Doc Samson, Lt. Cmdr. Geordi La Forge).
    for (text, name) in [
        (
            "If one or more +1/+1 counters would be put on a creature you control, that many plus one +1/+1 counters are put on it instead.",
            "Hardened Scales",
        ),
        (
            "If you would put one or more counters on a permanent you control, put that many plus one of each of those kinds of counters on that permanent instead.",
            "Doc Samson, Super Psychiatrist",
        ),
        (
            "If you would put one or more charge counters on an artifact you control, put twice that many charge counters on that artifact instead.",
            "Lt. Cmdr. Geordi La Forge",
        ),
    ] {
        let def = parse_replacement_line(text, name).unwrap();
        assert!(def.valid_card.is_some(), "{name}: permanent leg");
        assert_eq!(def.counter_player_recipient, None, "{name}: no player leg");
    }
}
