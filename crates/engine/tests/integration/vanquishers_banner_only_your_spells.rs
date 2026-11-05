//! Vanquisher's Banner: "Whenever you cast a creature spell of the chosen type,
//! draw a card." CR 603.2 + CR 109.5: "you" is the Banner's controller, so a
//! Knight spell an opponent casts draws nothing; one its controller casts draws.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::ability::ChosenAttribute;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;

/// Verbatim Oracle text (Scryfall).
const BANNER: &str = "As this artifact enters, choose a creature type.\n\
    Creatures you control of the chosen type get +1/+1.\n\
    Whenever you cast a creature spell of the chosen type, draw a card.";

/// P0's Banner naming Knight, a Knight in each player's hand, cards to draw.
fn board() -> (GameRunner, ObjectId, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let banner = scenario
        .add_artifact_from_oracle(P0, "Vanquisher's Banner", BANNER)
        .id();
    let ours = scenario
        .add_creature_to_hand(P0, "Our Knight", 2, 2)
        .with_subtypes(vec!["Knight"])
        .id();
    let theirs = scenario
        .add_creature_to_hand(P1, "Their Knight", 2, 2)
        .with_subtypes(vec!["Knight"])
        .flash()
        .id();
    for n in 0..5 {
        scenario.add_card_to_library_top(P0, &format!("Our card {n}"));
        scenario.add_card_to_library_top(P1, &format!("Their card {n}"));
    }
    let mut runner = scenario.build();
    runner
        .state_mut()
        .objects
        .get_mut(&banner)
        .unwrap()
        .chosen_attributes = vec![ChosenAttribute::CreatureType("Knight".to_string())];
    (runner, ours, theirs)
}

fn library(runner: &GameRunner, player: usize) -> usize {
    runner.state().players[player].library.len()
}

#[test]
fn vanquishers_banner_draws_for_our_knight_spell() {
    let (mut runner, ours, _) = board();
    runner.cast(ours).resolve();
    runner.advance_until_stack_empty();
    assert_eq!(library(&runner, 0), 4, "our Knight spell draws us a card");
}

#[test]
fn vanquishers_banner_ignores_an_opponents_knight_spell() {
    let (mut runner, _, theirs) = board();
    {
        let state = runner.state_mut();
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }
    runner.cast(theirs).resolve();
    runner.advance_until_stack_empty();
    assert_eq!(
        runner.state().objects[&theirs].zone,
        engine::types::zones::Zone::Battlefield,
        "the opponent's Knight was cast and resolved"
    );
    assert_eq!(library(&runner, 0), 5, "an opponent's Knight spell draws us nothing");
    assert_eq!(library(&runner, 1), 5, "nor its caster");
}
