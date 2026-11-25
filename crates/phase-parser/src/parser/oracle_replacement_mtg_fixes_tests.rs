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

/// Every `PutCounter` in an execute chain (outer link first), as upstream's
/// helper of the same name.
fn put_counters_in_chain(def: &ReplacementDefinition) -> Vec<(CounterType, QuantityExpr)> {
    let mut found = Vec::new();
    let mut cursor = def.execute.as_deref();
    while let Some(node) = cursor {
        if let Effect::PutCounter {
            counter_type,
            count,
            ..
        } = &*node.effect
        {
            found.push((counter_type.clone(), count.clone()));
        }
        cursor = node.sub_ability.as_deref();
    }
    found
}

/// The modifications of the self-affecting entry grant an enters-with
/// replacement carries in its execute chain, and the counters it places.
fn entry_grant_modifications(def: &ReplacementDefinition) -> Vec<ContinuousModification> {
    let mut link = def.execute.as_deref();
    let mut out = Vec::new();
    while let Some(ability) = link {
        if let Effect::GenericEffect {
            static_abilities, ..
        } = &*ability.effect
        {
            for static_def in static_abilities {
                assert_eq!(static_def.affected, Some(TargetFilter::SelfRef));
                out.extend(static_def.modifications.iter().cloned());
            }
        }
        link = ability.sub_ability.as_deref();
    }
    out
}

fn mutant() -> ContinuousModification {
    ContinuousModification::AddSubtype {
        subtype: "Mutant".to_string(),
    }
}

/// CR 614.1c + CR 205.1b + CR 122.6: Master Biomancer. "and as a Mutant in
/// addition to its other types" is a rider of the enters-with clause: the
/// counters are counted by the source's power, and the entrant gains the
/// Mutant subtype as it enters, for every other creature its controller controls.
#[test]
fn enters_with_counters_and_as_a_type_rider() {
    let def = parse_replacement_line(
        "Each other creature you control enters with a number of additional +1/+1 counters on it equal to this creature's power and as a Mutant in addition to its other types.",
        "Master Biomancer",
    )
    .expect("Master Biomancer parses");
    assert_eq!(def.event, ReplacementEvent::ChangeZone);
    assert_eq!(def.destination_zone, Some(Zone::Battlefield));
    let Some(TargetFilter::Typed(typed)) = &def.valid_card else {
        panic!("expected a typed subject, got {:?}", def.valid_card);
    };
    assert_eq!(typed.controller, Some(ControllerRef::You));
    assert!(typed.properties.contains(&FilterProp::Another));
    assert_eq!(entry_grant_modifications(&def), vec![mutant()]);
    let counters = put_counters_in_chain(&def);
    assert!(
        matches!(
            counters.as_slice(),
            [(
                CounterType::Plus1Plus1,
                QuantityExpr::Ref {
                    qty: QuantityRef::Power { .. }
                }
            )]
        ),
        "got {counters:?}"
    );
}

/// CR 614.1c + CR 205.1b: the rider after a printed count is read too, not
/// dropped, and a line without the rider grants no type.
#[test]
fn enters_with_type_rider_after_printed_count() {
    let def = parse_replacement_line(
        "Each other creature you control enters with an additional +1/+1 counter on it and as a Mutant in addition to its other types.",
        "Test Card",
    )
    .expect("parses");
    assert_eq!(entry_grant_modifications(&def), vec![mutant()]);
    assert_eq!(
        put_counters_in_chain(&def),
        vec![(CounterType::Plus1Plus1, QuantityExpr::Fixed { value: 1 })]
    );

    let plain = parse_replacement_line(
        "Each other creature you control enters with a number of additional +1/+1 counters on it equal to this creature's power.",
        "Test Card",
    )
    .expect("parses");
    assert!(entry_grant_modifications(&plain).is_empty());
}

/// CR 614.1c: a rider that is present but whose descriptor reads no type
/// fails the clause closed instead of claiming the counters alone.
#[test]
fn enters_with_unreadable_type_rider_fails_closed() {
    assert!(parse_replacement_line(
        "Each other creature you control enters with an additional +1/+1 counter on it and as a in addition to its other types.",
        "Test Card",
    )
    .is_none());
}
