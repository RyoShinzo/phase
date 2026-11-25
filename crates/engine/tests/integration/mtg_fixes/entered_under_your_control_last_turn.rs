//! CR 403.3 + CR 603.4 + CR 608.2i: "At the beginning of each upkeep, if you had
//! another creature enter the battlefield under your control last turn, draw a
//! card." (Ephara, God of the Polis). The condition reads the previous turn's
//! battlefield entries, latched at the turn boundary: an entry in the turn
//! before draws at the next upkeep, whichever player's it is, and a creature
//! that has left since still counts (Ephara's rulings).

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::zones::move_to_zone;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const UPKEEP_DRAW: &str = "At the beginning of each upkeep, if you had another creature \
    enter the battlefield under your control last turn, draw a card.";

fn hand_size(runner: &GameRunner) -> usize {
    runner.state().players[0].hand.len()
}

/// The upkeep trigger on an enchantment of player 0's, a creature card in each
/// player's hand, and cards to draw for both.
fn board() -> (GameRunner, ObjectId, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_enchantment_from_oracle(P0, "Polis Shrine", UPKEEP_DRAW);
    let ours = scenario.add_creature_to_hand(P0, "Our Bear", 2, 2).id();
    let theirs = scenario.add_creature_to_hand(P1, "Their Bear", 2, 2).id();
    for n in 0..10 {
        scenario.add_card_to_library_top(P0, &format!("Our card {n}"));
        scenario.add_card_to_library_top(P1, &format!("Their card {n}"));
    }
    (scenario.build(), ours, theirs)
}

/// Cards player 0 drew from the next upkeep's triggers. The way there leaves the
/// current step through a main phase, so that each call reaches a new upkeep.
fn drawn_at_next_upkeep(runner: &mut GameRunner) -> usize {
    runner.advance_to_phase(Phase::PreCombatMain);
    runner.advance_to_upkeep();
    let before = hand_size(runner);
    runner.advance_until_stack_empty();
    hand_size(runner) - before
}

/// A creature entered under our control in our turn: one card at the next
/// upkeep (player 1's), none at the upkeep after (nothing entered in player 1's
/// turn).
#[test]
fn an_entry_last_turn_draws_at_the_next_upkeep_only() {
    let (mut runner, ours, _) = board();
    runner.cast(ours).resolve();
    runner.advance_until_stack_empty();
    assert_eq!(drawn_at_next_upkeep(&mut runner), 1, "entered last turn");
    assert_eq!(runner.state().active_player, P1);
    assert_eq!(drawn_at_next_upkeep(&mut runner), 0, "nothing entered last turn");
    assert_eq!(runner.state().active_player, P0);
}

/// CR 608.2i: the creature that entered has left the battlefield before the
/// upkeep; it still entered last turn (Ephara's ruling).
#[test]
fn a_creature_that_left_still_counts() {
    let (mut runner, ours, _) = board();
    runner.cast(ours).resolve();
    runner.advance_until_stack_empty();
    let mut events = Vec::new();
    move_to_zone(runner.state_mut(), ours, Zone::Graveyard, &mut events);
    assert_eq!(drawn_at_next_upkeep(&mut runner), 1);
}

/// No creature entered last turn, or one entered under an opponent's control:
/// no card.
#[test]
fn no_entry_or_an_opponents_entry_draws_nothing() {
    let (mut runner, _, theirs) = board();
    assert_eq!(drawn_at_next_upkeep(&mut runner), 0, "nothing entered");
    runner.advance_to_phase(Phase::PreCombatMain);
    assert_eq!(runner.state().active_player, P1);
    runner.cast(theirs).resolve();
    runner.advance_until_stack_empty();
    assert_eq!(drawn_at_next_upkeep(&mut runner), 0, "an opponent's creature");
}

/// "another creature": the source's own entry does not count.
#[test]
fn the_sources_own_entry_does_not_count() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let source = scenario
        .add_creature_to_hand_from_oracle(P0, "Polis Herald", 2, 2, UPKEEP_DRAW)
        .id();
    for n in 0..10 {
        scenario.add_card_to_library_top(P0, &format!("Our card {n}"));
        scenario.add_card_to_library_top(P1, &format!("Their card {n}"));
    }
    let mut runner = scenario.build();
    runner.cast(source).resolve();
    runner.advance_until_stack_empty();
    assert_eq!(runner.state().objects[&source].zone, Zone::Battlefield);
    assert_eq!(drawn_at_next_upkeep(&mut runner), 0);
}
