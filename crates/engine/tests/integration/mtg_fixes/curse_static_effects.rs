//! Our tests beside upstream's file of the same name, in a file of our own so that a merge of
//! upstream never meets them. The helpers they share with upstream's file are copied from it.

use engine::game::effects::attach::attach_to_player;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameScenario, P0, P1};
use engine::types::phase::Phase;

const CURSE_OF_BLOODLETTING: &str =
    "If a source would deal damage to enchanted player, it deals double that damage to that player instead.";

/// Curse of Bloodletting: damage to a player it does not enchant is not doubled.
/// CR 303.4 + CR 614.1a: the shield covers only the enchanted player. The curse
/// enchants P0; P0's Bears deal their 2 to P1 unchanged.
#[test]
fn curse_of_bloodletting_leaves_other_players_damage_alone() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    let curse = {
        let mut builder = scenario.add_creature_from_oracle(
            P0,
            "Curse of Bloodletting",
            0,
            0,
            CURSE_OF_BLOODLETTING,
        );
        builder.as_enchantment();
        builder.with_subtypes(vec!["Aura", "Curse"]);
        builder.id()
    };
    let attacker = scenario.add_creature(P0, "Grizzly Bears", 2, 2).id();
    for _ in 0..20 {
        scenario.add_card_to_library_top(P0, "Plains");
        scenario.add_card_to_library_top(P1, "Plains");
    }

    let mut runner = scenario.build();
    attach_to_player(runner.state_mut(), curse, P0);
    evaluate_layers(runner.state_mut());

    runner.advance_to_combat();
    runner
        .declare_attackers(&[(attacker, engine::game::combat::AttackTarget::Player(P1))])
        .expect("DeclareAttackers must succeed");
    let life_before = runner.life(P1);
    runner.combat_damage();
    let damage_taken = life_before - runner.life(P1);

    assert_eq!(
        damage_taken, 2,
        "Curse of Bloodletting on P0 must not double damage to P1 (got {damage_taken})"
    );
}
