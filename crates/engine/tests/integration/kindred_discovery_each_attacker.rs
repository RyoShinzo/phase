//! CR 508.3a + CR 603.2c: "Whenever a creature you control of the chosen type
//! enters or attacks, draw a card." (Kindred Discovery) triggers once for each
//! creature of the chosen type declared as an attacker, not once per attack
//! declaration, and once for each such creature entering.

use engine::game::combat::AttackTarget;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::ability::ChosenAttribute;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;

const KINDRED_DISCOVERY: &str = "As this enchantment enters, choose a creature type.\n\
    Whenever a creature you control of the chosen type enters or attacks, draw a card.";

fn hand_size(runner: &GameRunner) -> usize {
    runner.state().players[0].hand.len()
}

/// Kindred Discovery naming Knight, two Knights and a Soldier on the battlefield,
/// a Knight and a Soldier in hand, and ten cards to draw.
fn board() -> (GameRunner, [ObjectId; 3], [ObjectId; 2]) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let discovery = scenario
        .add_enchantment_from_oracle(P0, "Kindred Discovery", KINDRED_DISCOVERY)
        .id();
    let knight_a = scenario
        .add_creature(P0, "Knight A", 2, 2)
        .with_subtypes(vec!["Knight"])
        .id();
    let knight_b = scenario
        .add_creature(P0, "Knight B", 2, 2)
        .with_subtypes(vec!["Human", "Knight"])
        .id();
    let soldier = scenario
        .add_creature(P0, "Soldier", 2, 2)
        .with_subtypes(vec!["Soldier"])
        .id();
    let new_knight = scenario
        .add_creature_to_hand(P0, "New Knight", 2, 2)
        .with_subtypes(vec!["Knight"])
        .id();
    let new_soldier = scenario
        .add_creature_to_hand(P0, "New Soldier", 2, 2)
        .with_subtypes(vec!["Soldier"])
        .id();
    for n in 0..10 {
        scenario.add_card_to_library_top(P0, &format!("Library card {n}"));
    }
    let mut runner = scenario.build();
    runner
        .state_mut()
        .objects
        .get_mut(&discovery)
        .unwrap()
        .chosen_attributes = vec![ChosenAttribute::CreatureType("Knight".to_string())];
    (runner, [knight_a, knight_b, soldier], [new_knight, new_soldier])
}

/// Two Knights and a Soldier attack in one declaration: two triggers, two cards.
#[test]
fn kindred_discovery_draws_for_each_attacking_creature_of_the_chosen_type() {
    let (mut runner, [knight_a, knight_b, soldier], _) = board();
    runner.advance_to_combat();
    let before = hand_size(&runner);
    runner
        .declare_attackers(&[
            (knight_a, AttackTarget::Player(P1)),
            (knight_b, AttackTarget::Player(P1)),
            (soldier, AttackTarget::Player(P1)),
        ])
        .expect("declare the three attackers");
    runner.advance_until_stack_empty();
    assert_eq!(
        hand_size(&runner) - before,
        2,
        "one draw per attacking Knight (CR 508.3a, CR 603.2c), none for the Soldier"
    );
}

/// A Knight entering draws a card; a Soldier entering does not.
#[test]
fn kindred_discovery_draws_for_each_entering_creature_of_the_chosen_type() {
    let (mut runner, _, [new_knight, new_soldier]) = board();
    let before = hand_size(&runner);
    runner.cast(new_knight).resolve();
    runner.advance_until_stack_empty();
    // The cast Knight left the hand, the trigger drew one.
    assert_eq!(hand_size(&runner), before, "the Knight entering draws one card");
    runner.cast(new_soldier).resolve();
    runner.advance_until_stack_empty();
    assert_eq!(
        hand_size(&runner),
        before - 1,
        "a creature of another type entering draws nothing"
    );
}
