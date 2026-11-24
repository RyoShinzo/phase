//! Our tests beside upstream's file of the same name, in a file of our own so that a merge of
//! upstream never meets them. The helpers they share with upstream's file are copied from it.

use engine::game::scenario::{GameScenario, P0};
use engine::types::phase::Phase;

const STRIDEHANGAR_REPLACEMENT: &str = "If one or more artifact tokens would be created under your control, those tokens plus an additional 1/1 colorless Thopter artifact creature token with flying are created instead.";
const TREASURE_MAKER_ETB: &str = "When this creature enters, create a Treasure token.";

fn tokens_with_subtype(runner: &engine::game::scenario::GameRunner, subtype: &str) -> usize {
    runner
        .state()
        .battlefield
        .iter()
        .filter(|id| {
            runner
                .state()
                .objects
                .get(id)
                .is_some_and(|o| o.is_token && o.card_types.subtypes.iter().any(|s| s == subtype))
        })
        .count()
}

const TWO_TREASURES_ETB: &str = "When this creature enters, create two Treasure tokens.";
const CHATTERFANG_REPLACEMENT: &str = "If one or more tokens would be created under your control, those tokens plus that many 1/1 green Squirrel creature tokens are created instead.";

/// Casts a free creature with `etb` beside the permanents `replacements` names, and
/// returns the runner once its trigger has resolved.
fn enter_with(replacements: &[(&str, &str)], etb: &str) -> engine::game::scenario::GameRunner {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    for (name, text) in replacements {
        scenario.add_creature_from_oracle(P0, name, 3, 3, text);
    }
    let maker = scenario
        .add_creature_to_hand_from_oracle(P0, "Token Maker", 1, 1, etb)
        .with_mana_cost(engine::types::mana::ManaCost::generic(0))
        .id();
    let mut runner = scenario.build();
    runner.cast(maker).resolve();
    runner.advance_until_stack_empty();
    runner
}

/// CR 614.1a: "those tokens plus an additional ... token" adds one Thopter to the
/// event, however many artifact tokens it creates (Xorn ruling: one additional
/// Treasure per copy of the replacement).
#[test]
fn stridehangar_adds_one_thopter_to_an_event_of_two_artifact_tokens() {
    let runner = enter_with(
        &[("Stridehangar Automaton", STRIDEHANGAR_REPLACEMENT)],
        TWO_TREASURES_ETB,
    );
    assert_eq!(tokens_with_subtype(&runner, "Treasure"), 2);
    assert_eq!(tokens_with_subtype(&runner, "Thopter"), 1);
}

/// CR 614.1a + CR 614.5: a second Automaton applies once to the one modified
/// event, Treasure and the first Automaton's Thopter alike, and adds one Thopter:
/// two in all, not one more for each batch of the event.
#[test]
fn two_stridehangars_add_one_thopter_each() {
    let automaton = ("Stridehangar Automaton", STRIDEHANGAR_REPLACEMENT);
    let runner = enter_with(&[automaton, automaton], TREASURE_MAKER_ETB);
    assert_eq!(tokens_with_subtype(&runner, "Treasure"), 1);
    assert_eq!(tokens_with_subtype(&runner, "Thopter"), 2);
}

/// CR 614.1a: "that many" stays the count of the tokens the replacement applies to.
#[test]
fn chatterfang_adds_that_many_squirrels_to_an_event_of_two_tokens() {
    let runner = enter_with(
        &[("Chatterfang, Squirrel General", CHATTERFANG_REPLACEMENT)],
        TWO_TREASURES_ETB,
    );
    assert_eq!(tokens_with_subtype(&runner, "Treasure"), 2);
    assert_eq!(tokens_with_subtype(&runner, "Squirrel"), 2);
}
