//! Holy Frazzle-Cannon (back face of Invasion of New Capenna): "Whenever equipped
//! creature attacks, put a +1/+1 counter on that creature and each other creature
//! you control that shares a creature type with it." CR 122.1 + CR 608.2c: the
//! attacker gets one counter, every other creature you control sharing a creature
//! type with it gets one, and nothing else does.

use engine::game::combat::AttackTarget;
use engine::game::effects::attach;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::counter::CounterType;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;

/// Verbatim Oracle text of the back face (Scryfall).
const HOLY_FRAZZLE_CANNON: &str = "Whenever equipped creature attacks, put a +1/+1 counter on that creature and each other creature you control that shares a creature type with it.\n\
    Equip {1}";

fn plus_counters(runner: &GameRunner, id: ObjectId) -> u32 {
    runner.state().objects[&id]
        .counters
        .get(&CounterType::Plus1Plus1)
        .copied()
        .unwrap_or(0)
}

#[test]
fn holy_frazzle_cannon_counters_the_attacker_and_each_creature_sharing_a_type() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let cannon = scenario
        .add_artifact_from_oracle(P0, "Holy Frazzle-Cannon", HOLY_FRAZZLE_CANNON)
        .with_subtypes(vec!["Equipment"])
        .id();
    let attacker = scenario
        .add_creature(P0, "Human Knight", 2, 2)
        .with_subtypes(vec!["Human", "Knight"])
        .id();
    let knight = scenario
        .add_creature(P0, "Spirit Knight", 2, 2)
        .with_subtypes(vec!["Spirit", "Knight"])
        .id();
    let human = scenario
        .add_creature(P0, "Human Soldier", 2, 2)
        .with_subtypes(vec!["Human", "Soldier"])
        .id();
    let zombie = scenario
        .add_creature(P0, "Zombie", 2, 2)
        .with_subtypes(vec!["Zombie"])
        .id();
    let their_knight = scenario
        .add_creature(P1, "Their Knight", 2, 2)
        .with_subtypes(vec!["Knight"])
        .id();
    let mut runner = scenario.build();
    // The creature-type registry a real game loads from the card data.
    runner.state_mut().all_creature_types = ["Human", "Knight", "Soldier", "Spirit", "Zombie"]
        .map(String::from)
        .to_vec();
    attach::attach_to(runner.state_mut(), cannon, attacker);

    runner.advance_to_combat();
    runner
        .declare_attackers(&[(attacker, AttackTarget::Player(P1))])
        .expect("declare the equipped Knight");
    runner.advance_until_stack_empty();

    assert_eq!(plus_counters(&runner, attacker), 1, "the attacker gets one counter, not two");
    assert_eq!(plus_counters(&runner, knight), 1, "shares Knight");
    assert_eq!(plus_counters(&runner, human), 1, "shares Human");
    assert_eq!(plus_counters(&runner, zombie), 0, "shares no type");
    assert_eq!(plus_counters(&runner, their_knight), 0, "not ours");
}
