//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `replacement.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::game::game_object::GameObject;
use crate::types::ability::{
    ControllerRef, QuantityModification, ReplacementDefinition, ReplacementPlayerScope,
    TargetFilter, TypeFilter, TypedFilter,
};
use crate::types::card_type::CoreType;
use crate::types::game_state::GameState;
use crate::types::identifiers::{CardId, ObjectId};
use crate::types::player::PlayerId;
use crate::types::replacements::ReplacementEvent;
use std::collections::HashSet;

fn test_state_with_object(
    obj_id: ObjectId,
    zone: Zone,
    replacements: Vec<ReplacementDefinition>,
) -> GameState {
    let mut state = GameState::new_two_player(42);
    let mut obj = GameObject::new(obj_id, CardId(1), PlayerId(0), "Test".to_string(), zone);
    obj.replacement_definitions = replacements.into();
    state.objects.insert(obj_id, obj);
    if zone == Zone::Battlefield {
        state.battlefield.push_back(obj_id);
    }
    state
}

/// CR 614.1a + CR 109.1 + CR 122.1: Lae'zel, Vlaakith's Champion — "If you
/// would put one or more counters on a creature or planeswalker you control
/// or on yourself, put that many plus one ... on that permanent or player
/// instead." The recipient is the permanents `valid_card` names or the
/// player `counter_player_recipient` names; "you would put" scopes the actor.
#[test]
fn counter_replacement_recipient_is_permanents_or_yourself() {
    let source = ObjectId(90);
    let laezel = |player_leg: Option<ReplacementPlayerScope>| {
        let mut repl = ReplacementDefinition::new(ReplacementEvent::AddCounter)
            .quantity_modification(QuantityModification::Plus { value: 1 })
            .valid_card(TargetFilter::Or {
                filters: vec![
                    TargetFilter::Typed(TypedFilter::creature().controller(ControllerRef::You)),
                    TargetFilter::Typed(
                        TypedFilter::new(TypeFilter::Planeswalker).controller(ControllerRef::You),
                    ),
                ],
            });
        repl.valid_player = Some(ReplacementPlayerScope::You);
        repl.counter_replacement_subject = crate::types::ability::CounterReplacementSubject::Actor;
        repl.counter_player_recipient = player_leg;
        repl
    };
    let mut state = test_state_with_object(
        source,
        Zone::Battlefield,
        vec![laezel(Some(ReplacementPlayerScope::You))],
    );
    let mut add_permanent = |id: u64, owner: u8, core: CoreType| {
        let id = ObjectId(id);
        let mut obj = GameObject::new(
            id,
            CardId(id.0),
            PlayerId(owner),
            format!("Permanent {}", id.0),
            Zone::Battlefield,
        );
        obj.card_types.core_types = vec![core];
        state.objects.insert(id, obj);
        state.battlefield.push_back(id);
        id
    };
    let own_creature = add_permanent(91, 0, CoreType::Creature);
    let own_planeswalker = add_permanent(92, 0, CoreType::Planeswalker);
    let own_artifact = add_permanent(93, 0, CoreType::Artifact);
    let opposing_creature = add_permanent(94, 1, CoreType::Creature);
    let registry = build_replacement_registry();

    let on_object = |actor: u8, object_id: ObjectId| ProposedEvent::AddCounter {
        placement: CounterPlacement::Object {
            actor: PlayerId(actor),
            object_id,
            counter_type: CounterType::Plus1Plus1,
            cause: crate::types::proposed_event::CounterCause::ResolvingEffect,
        },
        count: 1,
        applied: HashSet::new(),
    };
    let on_player = |actor: u8, player: u8| ProposedEvent::AddCounter {
        placement: CounterPlacement::Player {
            actor: PlayerId(actor),
            player_id: PlayerId(player),
            counter_kind: crate::types::player::PlayerCounterKind::Poison,
            cause: crate::types::proposed_event::CounterCause::ResolvingEffect,
        },
        count: 1,
        applied: HashSet::new(),
    };
    let applies = |state: &GameState, event: &ProposedEvent| {
        !find_applicable_replacements(state, event, &registry).is_empty()
    };

    assert!(applies(&state, &on_object(0, own_creature)), "our creature");
    assert!(
        applies(&state, &on_object(0, own_planeswalker)),
        "our planeswalker"
    );
    assert!(
        !applies(&state, &on_object(0, own_artifact)),
        "a noncreature artifact we control is not a creature or planeswalker"
    );
    assert!(
        !applies(&state, &on_object(0, opposing_creature)),
        "an opponent's creature is not one we control"
    );
    assert!(
        !applies(&state, &on_object(1, own_creature)),
        "an opponent putting counters is not \"you would put\""
    );
    assert!(applies(&state, &on_player(0, 0)), "on yourself");
    assert!(
        applies(
            &state,
            &ProposedEvent::AddCounter {
                placement: CounterPlacement::Energy {
                    actor: PlayerId(0),
                    player_id: PlayerId(0),
                    cause: crate::types::proposed_event::CounterCause::ResolvingEffect,
                },
                count: 1,
                applied: HashSet::new(),
            }
        ),
        "energy counters we get are counters on ourselves"
    );
    assert!(
        !applies(&state, &on_player(0, 1)),
        "an opponent is not yourself"
    );
    assert!(
        !applies(&state, &on_player(1, 0)),
        "an opponent putting counters on us is not \"you would put\""
    );

    let mut events = Vec::new();
    match replace_event(&mut state, on_player(0, 0), &mut events) {
        ReplacementResult::Execute(ProposedEvent::AddCounter { count, .. }) => {
            assert_eq!(count, 2, "that many plus one on yourself")
        }
        other => panic!("expected the poison placement to execute, got {other:?}"),
    }

    // Without the player leg the same permanent scope reaches no player.
    state
        .objects
        .get_mut(&source)
        .unwrap()
        .replacement_definitions = vec![laezel(None)].into();
    assert!(applies(&state, &on_object(0, own_creature)));
    assert!(
        !applies(&state, &on_player(0, 0)),
        "a permanent-only recipient never reaches a player"
    );
}
