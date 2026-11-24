//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `trigger_matchers.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::game::zones::create_object;
use crate::types::ability::{
    ControllerRef, TargetFilter, TriggerCondition, TriggerDefinition, TypedFilter,
};
use crate::types::card_type::CoreType;
use crate::types::events::GameEvent;
use crate::types::game_state::GameState;
use crate::types::identifiers::{CardId, ObjectId};
use crate::types::player::PlayerId;
use crate::types::zones::Zone;

/// Helper to create a minimal TriggerDefinition with typed fields.
fn make_trigger(mode: TriggerMode) -> TriggerDefinition {
    TriggerDefinition::new(mode)
}

/// CR 508.3d + CR 508.5a + CR 603.4: a player-subject attack trigger
/// ("Whenever an opponent attacks with creatures", Mangara, the Diplomat)
/// fires once per attacked player, and each firing carries every attacker
/// grouped under that player: one at the player and one at their
/// planeswalker share a firing, one at another player has its own. The
/// intervening-if "two or more of those creatures are attacking you and/or
/// planeswalkers you control" holds for the first firing only.
#[test]
fn player_scope_attack_trigger_groups_attackers_by_defending_player() {
    let mut state = GameState::new(crate::types::format::FormatConfig::free_for_all(), 3, 42);
    let mangara = create_object(
        &mut state,
        CardId(1),
        PlayerId(0),
        "Mangara".to_string(),
        Zone::Battlefield,
    );
    let planeswalker = create_object(
        &mut state,
        CardId(2),
        PlayerId(0),
        "Planeswalker".to_string(),
        Zone::Battlefield,
    );
    let attackers: Vec<ObjectId> = (0..3)
        .map(|i| {
            let id = create_object(
                &mut state,
                CardId(10 + i),
                PlayerId(1),
                format!("Attacker {i}"),
                Zone::Battlefield,
            );
            state
                .objects
                .get_mut(&id)
                .unwrap()
                .card_types
                .core_types
                .push(CoreType::Creature);
            id
        })
        .collect();

    let mut trigger = make_trigger(TriggerMode::Attacks);
    trigger.valid_source = Some(TargetFilter::Typed(
        TypedFilter::default().controller(ControllerRef::Opponent),
    ));
    let event = GameEvent::AttackersDeclared {
        attacker_ids: attackers.clone(),
        defending_player: PlayerId(0),
        attacks: vec![
            (
                attackers[0],
                crate::game::combat::AttackTarget::Player(PlayerId(0)),
            ),
            (
                attackers[1],
                crate::game::combat::AttackTarget::Player(PlayerId(2)),
            ),
            (
                attackers[2],
                crate::game::combat::AttackTarget::Planeswalker(planeswalker),
            ),
        ],
        declaration_records: Vec::new(),
    };

    let matched = matching_attack_events(
        &event,
        &trigger,
        &test_trigger_source_context(&state, mangara),
        &state,
    );
    let groups: Vec<(PlayerId, Vec<ObjectId>)> = matched
        .iter()
        .map(|event| match event {
            GameEvent::AttackersDeclared {
                attacker_ids,
                defending_player,
                ..
            } => (*defending_player, attacker_ids.clone()),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        groups,
        vec![
            (PlayerId(0), vec![attackers[0], attackers[2]]),
            (PlayerId(2), vec![attackers[1]]),
        ]
    );

    let condition = TriggerCondition::AttackersDeclaredCount {
        subject: crate::types::ability::AttackersDeclaredCountSubject::AttackTarget {
            controller: ControllerRef::You,
            attacked: crate::types::triggers::AttackTargetFilter::PlayerOrPlaneswalker,
            filter: None,
        },
        comparator: crate::types::ability::Comparator::GE,
        count: 2,
    };
    let holds: Vec<bool> = matched
        .iter()
        .map(|event| {
            crate::game::triggers::check_trigger_condition(
                &state,
                &condition,
                PlayerId(0),
                Some(mangara),
                Some(event),
            )
        })
        .collect();
    assert_eq!(holds, vec![true, false]);
}
