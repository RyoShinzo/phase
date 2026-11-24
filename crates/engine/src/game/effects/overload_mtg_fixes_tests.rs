//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `overload.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::ability::{
    AbilityDefinition, AbilityKind, Effect, TargetFilter, TypeFilter, TypedFilter,
};
use crate::types::Zone;

/// Winds of Abandon's ability as the parser builds it.
const WINDS_OF_ABANDON: &str = r#"{"condition": null, "cost": null, "description": "Exile target creature you don't control. For each creature exiled this way, its controller searches their library for a basic land card. Those players put those cards onto the battlefield tapped, then shuffle.", "duration": null, "effect": {"destination": "Exile", "enter_tapped": false, "enter_transformed": false, "enters_attacking": false, "origin": null, "owner_library": false, "target": {"controller": "Opponent", "properties": [], "type": "Typed", "type_filters": ["Creature"]}, "type": "ChangeZone"}, "forward_result": false, "kind": "Spell", "optional": false, "optional_targeting": false, "sub_ability": {"condition": null, "cost": null, "description": null, "duration": null, "effect": {"count": {"type": "Fixed", "value": 1}, "filter": {"controller": null, "properties": [{"type": "HasSupertype", "value": "Basic"}], "type": "Typed", "type_filters": ["Land"]}, "reveal": false, "target_player": {"type": "ParentTargetController"}, "type": "SearchLibrary"}, "forward_result": false, "kind": "Spell", "optional": false, "optional_targeting": false, "repeat_for": {"qty": {"caused_by": "Exiled", "filter": {"controller": null, "properties": [], "type": "Typed", "type_filters": ["Creature"]}, "type": "FilteredTrackedSetSize"}, "type": "Ref"}, "sub_ability": {"condition": null, "cost": null, "description": null, "duration": null, "effect": {"destination": "Battlefield", "enter_tapped": true, "enter_transformed": false, "enters_attacking": false, "origin": "Library", "owner_library": false, "target": {"type": "Any"}, "type": "ChangeZone"}, "forward_result": false, "kind": "Spell", "optional": false, "optional_targeting": false, "sub_ability": {"condition": null, "cost": null, "description": null, "duration": null, "effect": {"target": {"type": "ParentTargetController"}, "type": "Shuffle"}, "forward_result": false, "kind": "Spell", "optional": false, "optional_targeting": false, "sub_ability": null, "target_prompt": null}, "target_prompt": null}, "sub_link": "SequentialSibling", "target_prompt": null}, "target_prompt": null}"#;

fn grant(target: Option<TargetFilter>, affected: TargetFilter) -> AbilityDefinition {
    use crate::types::ability::{ContinuousModification, Duration, StaticDefinition};
    use crate::types::keywords::Keyword;
    leaf(Effect::GenericEffect {
        static_abilities: vec![StaticDefinition::continuous()
            .affected(affected)
            .modifications(vec![ContinuousModification::AddKeyword {
                keyword: Keyword::Flying,
            }])],
        duration: Some(Duration::UntilEndOfTurn),
        target,
        end_cost: None,
    })
}

fn creature_filter() -> TargetFilter {
    TargetFilter::Typed(TypedFilter {
        type_filters: vec![TypeFilter::Creature],
        controller: None,
        properties: vec![],
    })
}

fn leaf(effect: Effect) -> AbilityDefinition {
    AbilityDefinition::new(AbilityKind::Spell, effect)
}

/// CR 702.96a + CR 601.2c: Winds of Abandon overloaded — the exile
/// promotes to `ChangeZoneAll`, but "Those players put those cards onto
/// the battlefield tapped" moves the searched cards and announces no
/// target, so it stays a single `ChangeZone` of the chosen cards (still
/// entering tapped) and its shuffle is untouched. Promoting it would put
/// every card of every library onto the battlefield. The tree is the
/// parser's output for the card.
#[test]
fn move_of_searched_cards_not_promoted() {
    let mut def: AbilityDefinition = serde_json::from_str(WINDS_OF_ABANDON).expect("tree");
    transform_ability_def(&mut def);
    assert!(
        matches!(
            *def.effect,
            Effect::ChangeZoneAll {
                destination: Zone::Exile,
                ..
            }
        ),
        "the exile promotes, got {:?}",
        def.effect
    );
    let search = def.sub_ability.as_ref().expect("search");
    assert!(matches!(*search.effect, Effect::SearchLibrary { .. }));
    let put = search.sub_ability.as_ref().expect("put");
    assert!(
        matches!(
            *put.effect,
            Effect::ChangeZone {
                destination: Zone::Battlefield,
                enter_tapped: crate::types::zones::EtbTapState::Tapped,
                ..
            }
        ),
        "the searched cards' move stays single, got {:?}",
        put.effect
    );
    let shuffle = put.sub_ability.as_ref().expect("shuffle");
    assert!(matches!(*shuffle.effect, Effect::Shuffle { .. }));
}

#[test]
fn targeted_grant_becomes_grant_to_each() {
    // CR 702.96a + CR 702.96b: "Target creature gains …" overloaded is
    // "Each creature gains …": no targeting slot, the filter is the population.
    let mut def = grant(Some(creature_filter()), TargetFilter::ParentTarget);
    transform_ability_def(&mut def);
    match &*def.effect {
        Effect::GenericEffect {
            target,
            static_abilities,
            ..
        } => {
            assert!(target.is_none(), "overloaded grant has no target");
            assert_eq!(static_abilities[0].affected, Some(creature_filter()));
        }
        other => panic!("expected GenericEffect, got {other:?}"),
    }
}

#[test]
fn untargeted_grant_unchanged() {
    // A grant to the source or an untargeted population has no "target".
    for (target, affected) in [
        (None, creature_filter()),
        (Some(TargetFilter::SelfRef), TargetFilter::ParentTarget),
        (Some(creature_filter()), TargetFilter::TriggeringSource),
    ] {
        let mut def = grant(target.clone(), affected.clone());
        transform_ability_def(&mut def);
        match &*def.effect {
            Effect::GenericEffect {
                target: after,
                static_abilities,
                ..
            } => {
                assert_eq!(after, &target);
                assert_eq!(static_abilities[0].affected, Some(affected));
            }
            other => panic!("expected GenericEffect, got {other:?}"),
        }
    }
}
