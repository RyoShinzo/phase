//! Automatic payment of a cost whose plan funds a costed mana ability's
//! sub-cost from a source the plan left free.
//!
//! Repro (game 5 of a Jean-Luc Picard, Luminary list): Aurelia, the Warleader
//! (`{2}{R}{R}{W}{W}`) with two Mountains, two Plains, a colorless rock and a
//! Boros Signet (`{1}, {T}: Add {R}{W}`). The plan taps the Signet for `{R}{W}`,
//! a Mountain and a Plains for the other colored shards, the rock and the second
//! Mountain for `{2}`, and leaves the second Plains to fund the Signet's `{1}`.
//! Phase 3 taps the plain sources first, so `{C}` floats when the Signet's
//! sub-cost is paid. The sub-cost was paid from that `{C}`, the outer cost's
//! generic mana, because only the outer cost's colored shards were reserved.
//! The outer cost was then one short with a Plains untapped, the automatic
//! payment fell back to manual payment, and the player tapped by hand.
//!
//! CR 107.4b: generic mana may be paid with any type, but each unit pays only
//! one cost (CR 118.10), so mana the outer plan counts on for its generic part
//! is as reserved as mana for its colored shards.

use engine::game::scenario::{GameScenario, P0};
use engine::types::actions::GameAction;
use engine::types::card_type::CoreType;
use engine::types::game_state::{CastPaymentMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const BOROS_SIGNET_ORACLE: &str = "{1}, {T}: Add {R}{W}.";
const MIND_STONE_ORACLE: &str = "{T}: Add {C}.";
const TALISMAN_OF_CREATIVITY_ORACLE: &str =
    "{T}: Add {C}.\n{T}: Add {U} or {R}. This artifact deals 1 damage to you.";

fn aurelia_cost() -> ManaCost {
    ManaCost::Cost {
        shards: vec![
            ManaCostShard::Red,
            ManaCostShard::Red,
            ManaCostShard::White,
            ManaCostShard::White,
        ],
        generic: 2,
    }
}

fn make_artifact(runner: &mut engine::game::scenario::GameRunner, id: ObjectId) {
    let obj = runner.state_mut().objects.get_mut(&id).unwrap();
    obj.card_types.core_types = vec![CoreType::Artifact];
    obj.base_card_types = obj.card_types.clone();
    obj.power = None;
    obj.toughness = None;
    obj.base_power = None;
    obj.base_toughness = None;
}

/// Casts a `{2}{R}{R}{W}{W}` spell with Auto payment from two Mountains, two
/// Plains, the given colorless rock and a Boros Signet, and checks that the
/// payment completes on its own with every source tapped.
fn auto_pays_with_signet_and_rock(rock_name: &str, rock_oracle: &str) {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    let lands: Vec<ObjectId> = [
        ManaColor::Red,
        ManaColor::Red,
        ManaColor::White,
        ManaColor::White,
    ]
    .into_iter()
    .map(|color| scenario.add_basic_land(P0, color))
    .collect();
    let rock = scenario
        .add_creature_from_oracle(P0, rock_name, 0, 0, rock_oracle)
        .id();
    let signet = scenario
        .add_creature_from_oracle(P0, "Boros Signet", 0, 0, BOROS_SIGNET_ORACLE)
        .id();
    let spell = scenario
        .add_creature_to_hand_from_oracle(P0, "Aurelia, the Warleader", 3, 4, "Flying")
        .with_mana_cost(aurelia_cost())
        .id();
    let mut runner = scenario.build();
    make_artifact(&mut runner, rock);
    make_artifact(&mut runner, signet);
    let card_id = runner.state().objects[&spell].card_id;

    runner
        .act(GameAction::CastSpell {
            object_id: spell,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("the automatic cast is accepted");

    assert!(
        !matches!(runner.state().waiting_for, WaitingFor::ManaPayment { .. }),
        "six sources make exactly {{2}}{{R}}{{R}}{{W}}{{W}}, so Auto pays it without \
         manual payment; waiting for {:?}",
        runner.state().waiting_for
    );
    assert_eq!(
        runner.state().objects[&spell].zone,
        Zone::Stack,
        "the spell was cast"
    );
    for id in lands.iter().chain([&rock, &signet]) {
        assert!(
            runner.state().objects[id].tapped,
            "{} was tapped for the cost",
            runner.state().objects[id].name
        );
    }
    assert_eq!(
        runner.state().players[0].mana_pool.total(),
        0,
        "every unit paid the spell or the Signet"
    );
    assert_eq!(runner.state().players[0].life, 20, "no pain mana was needed");
}

#[test]
fn signet_sub_cost_does_not_take_floating_generic_mana_mind_stone() {
    auto_pays_with_signet_and_rock("Mind Stone", MIND_STONE_ORACLE);
}

#[test]
fn signet_sub_cost_does_not_take_floating_generic_mana_talisman() {
    auto_pays_with_signet_and_rock("Talisman of Creativity", TALISMAN_OF_CREATIVITY_ORACLE);
}

/// The other side of the rule: when the selected rows make more mana than the
/// cost, the surplus still floats for the Signet's `{1}` and no further source is
/// tapped. `{1}{R}{W}` from Sol Ring and Boros Signet leaves the Plains untapped.
#[test]
fn signet_sub_cost_uses_surplus_mana_and_leaves_a_land_untapped() {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    let plains = scenario.add_basic_land(P0, ManaColor::White);
    let sol_ring = scenario
        .add_creature_from_oracle(P0, "Sol Ring", 0, 0, "{T}: Add {C}{C}.")
        .id();
    let signet = scenario
        .add_creature_from_oracle(P0, "Boros Signet", 0, 0, BOROS_SIGNET_ORACLE)
        .id();
    let spell = scenario
        .add_creature_to_hand_from_oracle(P0, "Commander William Riker", 3, 3, "Vigilance")
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Red, ManaCostShard::White],
            generic: 1,
        })
        .id();
    let mut runner = scenario.build();
    make_artifact(&mut runner, sol_ring);
    make_artifact(&mut runner, signet);
    let card_id = runner.state().objects[&spell].card_id;

    runner
        .act(GameAction::CastSpell {
            object_id: spell,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("the automatic cast is accepted");

    assert_eq!(runner.state().objects[&spell].zone, Zone::Stack, "the spell was cast");
    assert!(runner.state().objects[&sol_ring].tapped, "Sol Ring paid {{1}} and the Signet's {{1}}");
    assert!(runner.state().objects[&signet].tapped, "Boros Signet paid {{R}}{{W}}");
    assert!(
        !runner.state().objects[&plains].tapped,
        "Sol Ring's second {{C}} paid the Signet, so the Plains stays untapped"
    );
}
