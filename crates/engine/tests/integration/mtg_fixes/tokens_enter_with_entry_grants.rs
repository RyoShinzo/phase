//! CR 614.1c + CR 611.2a + CR 111.1: an "enters with … and as a <type> in
//! addition to its other types" replacement (Master Biomancer) applies to tokens
//! as to any other creature entering: each token of a batch enters with the
//! counters and with the type, and a token that's a copy (CR 707.2: neither the
//! counters nor the type are copied) gets them from the replacement as it enters.
//! The type is the entrant's own grant, never the replacement's host's.

use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::counter::CounterType;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const BIOMANCER: &str = "Each other creature you control enters with a number of additional \
    +1/+1 counters on it equal to this creature's power and as a Mutant in addition to its \
    other types.";
const SOLDIERS: &str = "{T}: Create two 1/1 white Soldier creature tokens.";
const COPIER: &str = "{T}: For each creature token you control, create a token that's a copy \
    of that creature.";

fn tokens(runner: &GameRunner) -> Vec<ObjectId> {
    let state = runner.state();
    state
        .battlefield
        .iter()
        .copied()
        .filter(|id| state.objects[id].is_token)
        .collect()
}

fn counters(runner: &GameRunner, id: ObjectId) -> u32 {
    runner.state().objects[&id]
        .counters
        .get(&CounterType::Plus1Plus1)
        .copied()
        .unwrap_or(0)
}

fn is_mutant(runner: &GameRunner, id: ObjectId) -> bool {
    runner.state().objects[&id]
        .card_types
        .subtypes
        .iter()
        .any(|subtype| subtype == "Mutant")
}

fn is_soldier(runner: &GameRunner, id: ObjectId) -> bool {
    runner.state().objects[&id]
        .card_types
        .subtypes
        .iter()
        .any(|subtype| subtype == "Soldier")
}

fn activate(runner: &mut GameRunner, source: ObjectId) {
    runner.activate(source, 0).resolve();
    runner.advance_until_stack_empty();
}

/// The setup: the replacement's host (2/4) on the battlefield, then the token
/// maker and the copier, which entered before it and so are unchanged.
fn setup() -> (GameRunner, ObjectId, ObjectId, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let maker = scenario
        .add_creature_from_oracle(P0, "Soldier Maker", 1, 1, SOLDIERS)
        .id();
    let copier = scenario
        .add_creature_from_oracle(P0, "Token Copier", 1, 1, COPIER)
        .id();
    let host = scenario
        .add_creature_from_oracle(P0, "Biomancer", 2, 4, BIOMANCER)
        .id();
    (scenario.build(), host, maker, copier)
}

#[test]
fn each_token_of_a_batch_enters_with_the_counters_and_the_type() {
    let (mut runner, host, maker, _) = setup();
    assert!(
        !is_mutant(&runner, maker),
        "a creature that entered before the host"
    );
    activate(&mut runner, maker);
    let soldiers = tokens(&runner);
    assert_eq!(soldiers.len(), 2, "the batch is created, not replaced away");
    for &soldier in &soldiers {
        assert_eq!(
            counters(&runner, soldier),
            2,
            "the host's power in counters"
        );
        assert!(is_mutant(&runner, soldier), "enters as a Mutant");
        assert!(
            is_soldier(&runner, soldier),
            "in addition to its other types"
        );
    }
    assert_eq!(
        counters(&runner, host),
        0,
        "the host gets none of the counters"
    );
    assert!(!is_mutant(&runner, host), "nor the type");
}

#[test]
fn a_token_copy_enters_with_the_counters_and_the_type_of_its_own() {
    let (mut runner, host, maker, copier) = setup();
    activate(&mut runner, maker);
    let originals = tokens(&runner);
    activate(&mut runner, copier);
    let copies: Vec<ObjectId> = tokens(&runner)
        .into_iter()
        .filter(|id| !originals.contains(id))
        .collect();
    assert_eq!(copies.len(), 2, "one copy of each token");
    for &copy in &copies {
        assert_eq!(
            counters(&runner, copy),
            2,
            "the copy's own counters, not copied ones"
        );
        assert!(is_mutant(&runner, copy), "the copy enters as a Mutant");
        assert!(is_soldier(&runner, copy), "and is a Soldier as copied");
    }
    assert_eq!(counters(&runner, host), 0);
    assert!(!is_mutant(&runner, host));
    assert!(!is_mutant(&runner, copier), "the copier is no entrant");
}

/// CR 611.2a: the type is the entrant's for as long as it stays on the
/// battlefield, so it outlasts the replacement's host.
#[test]
fn the_type_outlasts_the_host() {
    let (mut runner, host, maker, _) = setup();
    activate(&mut runner, maker);
    let soldiers = tokens(&runner);
    engine::game::zones::move_to_zone(runner.state_mut(), host, Zone::Graveyard, &mut Vec::new());
    engine::game::layers::evaluate_layers(runner.state_mut());
    assert_eq!(runner.state().objects[&host].zone, Zone::Graveyard);
    for &soldier in &soldiers {
        assert!(
            is_mutant(&runner, soldier),
            "still a Mutant without the host"
        );
    }
}
