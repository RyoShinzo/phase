//! Counters put on several creatures at once, each asking the order of two counter
//! replacements (CR 616.1e): Ozolith, the Shattered Spire and The Earth Crystal.
//!
//! Each answer to the order prompt places one creature's counters and stops at the next
//! creature's prompt. An action that ends on a prompt is never scanned for triggers, so
//! Shalai and Hallar's "Whenever one or more +1/+1 counters are put on a creature you
//! control" triggered for the last creature only. Every placement is a separate event
//! (CR 603.2, CR 603.2c) and triggers once, and each `CounterAdded` reaches the event
//! stream once.

use engine::game::scenario::{GameScenario, P0, P1};
use engine::types::actions::GameAction;
use engine::types::counter::CounterType;
use engine::types::events::GameEvent;
use engine::types::game_state::{GameState, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::ability::TargetRef;

const SHALAI_AND_HALLAR: &str = "Flying, vigilance\n\
Whenever one or more +1/+1 counters are put on a creature you control, \
Shalai and Hallar deals that much damage to target opponent.";
const OZOLITH: &str = "If one or more +1/+1 counters would be put on an artifact or creature \
you control, that many plus one +1/+1 counters are put on it instead.";
const EARTH_CRYSTAL: &str = "If one or more +1/+1 counters would be put on a creature you \
control, twice that many +1/+1 counters are put on that creature instead.";
const TEAM_COUNTER: &str = "Put a +1/+1 counter on each creature you control.";

fn p1p1(state: &GameState, id: ObjectId) -> u32 {
    state.objects[&id]
        .counters
        .get(&CounterType::Plus1Plus1)
        .copied()
        .unwrap_or(0)
}

#[test]
fn every_recipient_of_a_paused_counter_batch_triggers_once() {
    for ozolith_first in [true, false] {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        let shalai = scenario
            .add_creature_from_oracle(P0, "Shalai and Hallar", 3, 3, SHALAI_AND_HALLAR)
            .id();
        let bears = scenario.add_creature(P0, "Grizzly Bears", 2, 2).id();
        scenario.add_artifact_from_oracle(P0, "Ozolith, the Shattered Spire", OZOLITH);
        scenario.add_artifact_from_oracle(P0, "The Earth Crystal", EARTH_CRYSTAL);
        let spell = scenario
            .add_spell_to_hand_from_oracle(P0, "Team Counter", false, TEAM_COUNTER)
            .id();
        let mut runner = scenario.build();
        let life_before = runner.state().players[P1.0 as usize].life;

        runner.cast(spell).commit();
        let mut events: Vec<GameEvent> = Vec::new();
        let mut order_prompts = 0;
        for _ in 0..40 {
            let action = match &runner.state().waiting_for {
                WaitingFor::ReplacementChoice { candidates, .. } => {
                    order_prompts += 1;
                    let wanted = if ozolith_first { "Ozolith" } else { "The Earth" };
                    let index = candidates
                        .iter()
                        .position(|c| c.source_name.starts_with(wanted))
                        .expect("both replacements are offered");
                    GameAction::ChooseReplacement { index }
                }
                WaitingFor::TriggerTargetSelection { .. } => GameAction::ChooseTarget {
                    target: Some(TargetRef::Player(P1)),
                },
                WaitingFor::Priority { .. } if !runner.state().stack.is_empty() => {
                    GameAction::PassPriority
                }
                WaitingFor::OrderTriggers { triggers, .. } => GameAction::OrderTriggers {
                    order: (0..triggers.len()).collect(),
                },
                _ => break,
            };
            events.extend(runner.act(action).expect("the action is accepted").events);
        }

        // Ozolith first: (1 + 1) × 2 = 4; The Earth Crystal first: 1 × 2 + 1 = 3.
        let each = if ozolith_first { 4 } else { 3 };
        assert_eq!(order_prompts, 2, "each creature asks the order once");
        assert_eq!((p1p1(runner.state(), shalai), p1p1(runner.state(), bears)), (each, each));
        assert_eq!(
            runner.state().players[P1.0 as usize].life,
            life_before - 2 * each as i32,
            "Shalai and Hallar triggers once for each creature"
        );
        let placed: Vec<ObjectId> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::CounterAdded {
                    object_id,
                    counter_type: CounterType::Plus1Plus1,
                    ..
                } => Some(*object_id),
                _ => None,
            })
            .collect();
        assert_eq!(placed.len(), 2, "one placement event per creature: {placed:?}");
        assert!(placed.contains(&shalai) && placed.contains(&bears));
    }
}
