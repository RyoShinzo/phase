//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `engine_station_tests.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::game::zones::create_object;
use crate::types::card_type::CoreType;
use crate::types::counter::CounterType;
use crate::types::identifiers::{CardId, ObjectId};
use crate::types::player::PlayerId;
use crate::types::zones::Zone;

/// Set up a Spacecraft with the Station keyword and two eligible creatures.
fn setup_station_scenario() -> (GameState, ObjectId, ObjectId, ObjectId) {
    let mut state = setup_game_at_main_phase();

    let spacecraft_id = create_object(
        &mut state,
        CardId(300),
        PlayerId(0),
        "Test Spacecraft".to_string(),
        Zone::Battlefield,
    );
    {
        let obj = state.objects.get_mut(&spacecraft_id).unwrap();
        obj.card_types.core_types.push(CoreType::Artifact);
        obj.card_types.subtypes.push("Spacecraft".to_string());
        obj.keywords.push(crate::types::keywords::Keyword::Station);
    }

    let power_5 = create_object(
        &mut state,
        CardId(301),
        PlayerId(0),
        "Power 5 Creature".to_string(),
        Zone::Battlefield,
    );
    {
        let obj = state.objects.get_mut(&power_5).unwrap();
        obj.card_types.core_types.push(CoreType::Creature);
        obj.power = Some(5);
        obj.toughness = Some(5);
        obj.base_power = Some(5);
        obj.base_toughness = Some(5);
    }

    let power_2 = create_object(
        &mut state,
        CardId(302),
        PlayerId(0),
        "Power 2 Creature".to_string(),
        Zone::Battlefield,
    );
    {
        let obj = state.objects.get_mut(&power_2).unwrap();
        obj.card_types.core_types.push(CoreType::Creature);
        obj.power = Some(2);
        obj.toughness = Some(2);
        obj.base_power = Some(2);
        obj.base_toughness = Some(2);
    }

    (state, spacecraft_id, power_5, power_2)
}

fn setup_game_at_main_phase() -> GameState {
    let mut state = new_game(42);
    state.turn_number = 2;
    state.phase = Phase::PreCombatMain;
    state.active_player = PlayerId(0);
    state.priority_player = PlayerId(0);
    state.waiting_for = WaitingFor::Priority {
        player: PlayerId(0),
    };
    state
}

/// CR 702.26b: a phased-out Spacecraft's station can't be activated, a
/// phased-out creature can't be tapped to station, and a Spacecraft that
/// phases out before its station ability resolves gets no counters.
#[test]
fn station_ignores_phased_out_permanents() {
    use crate::game::game_object::PhaseOutCause;
    let (mut state, spacecraft_id, p5, p2) = setup_station_scenario();
    let mut events = Vec::new();
    crate::game::phasing::phase_out_object(&mut state, p5, PhaseOutCause::Directly, &mut events);
    let result = apply_as_current(
        &mut state,
        GameAction::ActivateStation {
            spacecraft_id,
            creature_id: None,
        },
    )
    .unwrap();
    match result.waiting_for {
        WaitingFor::StationTarget {
            eligible_creatures, ..
        } => {
            assert_eq!(
                eligible_creatures,
                vec![p2],
                "a phased-out creature can't station"
            );
        }
        other => panic!("Expected StationTarget, got {other:?}"),
    }
    apply_as_current(
        &mut state,
        GameAction::ActivateStation {
            spacecraft_id,
            creature_id: Some(p2),
        },
    )
    .unwrap();
    crate::game::phasing::phase_out_object(
        &mut state,
        spacecraft_id,
        PhaseOutCause::Directly,
        &mut events,
    );
    crate::game::stack::resolve_top(&mut state, &mut events);
    assert_eq!(
        state.objects[&spacecraft_id]
            .counters
            .get(&CounterType::Generic("charge".to_string()))
            .copied()
            .unwrap_or(0),
        0,
        "a phased-out Spacecraft can't get charge counters"
    );

    let (mut state, spacecraft_id, _, _) = setup_station_scenario();
    crate::game::phasing::phase_out_object(
        &mut state,
        spacecraft_id,
        PhaseOutCause::Directly,
        &mut events,
    );
    let rejected = apply_as_current(
        &mut state,
        GameAction::ActivateStation {
            spacecraft_id,
            creature_id: None,
        },
    );
    assert!(
        matches!(rejected, Err(EngineError::ActionNotAllowed(_))),
        "a phased-out Spacecraft can't station, got {rejected:?}"
    );
}
