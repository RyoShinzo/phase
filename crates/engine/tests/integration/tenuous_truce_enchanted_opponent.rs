//! Tenuous Truce — "Enchant opponent / At the beginning of enchanted opponent's
//! end step, you and that player each draw a card. / When you attack enchanted
//! opponent or a planeswalker they control or when they attack you or a
//! planeswalker you control, sacrifice this Aura."
//!
//! Before the fix "enchanted opponent's end step" had no turn scope, so it drew at
//! every player's end step; "you attack enchanted opponent" was a bare
//! `YouAttack`, so any attack sacrificed the Aura; and the second half ("they
//! attack you …") was an `Unknown` trigger that never fired.
//!
//! CR 303.4b: an Aura enchanting a player is attached to that player.
//! CR 508.1b: a planeswalker's controller is the player it defends for.
//! CR 603.2: "when A or when B" triggers on either event.

use engine::game::combat::AttackTarget;
use engine::game::effects::attach::attach_to_player;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::trigger_index::reindex_object_triggers;
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);

const TENUOUS_TRUCE: &str = "Enchant opponent\n\
     At the beginning of enchanted opponent's end step, you and that player each draw a card.\n\
     When you attack enchanted opponent or a planeswalker they control or when they attack you or a planeswalker you control, sacrifice this Aura.";

/// Four players, P0's Tenuous Truce enchanting P2, a Grizzly Bears for every
/// player and a planeswalker for every player, ten cards in every library.
/// Returns the runner, the Aura, the creatures and the planeswalkers by player.
fn table() -> (GameRunner, ObjectId, Vec<ObjectId>, Vec<ObjectId>) {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    let truce = {
        let mut builder = scenario.add_creature(P0, "Tenuous Truce", 0, 0);
        builder.as_enchantment();
        builder.with_subtypes(vec!["Aura"]);
        builder.from_oracle_text(TENUOUS_TRUCE);
        builder.id()
    };
    let mut creatures = Vec::new();
    let mut walkers = Vec::new();
    for player in 0..4u8 {
        let player = PlayerId(player);
        creatures.push(scenario.add_creature(player, "Grizzly Bears", 2, 2).id());
        walkers.push(
            scenario
                .add_planeswalker_from_oracle(player, "Test Walker", "Test", 5, "+1: You gain 1 life.")
                .id(),
        );
        for _ in 0..10 {
            scenario.add_card_to_library_top(player, "Plains");
        }
    }
    let mut runner = scenario.build();
    attach_to_player(runner.state_mut(), truce, P2);
    evaluate_layers(runner.state_mut());
    reindex_object_triggers(runner.state_mut(), truce);
    (runner, truce, creatures, walkers)
}

/// Make `player` the active player and pass priority until its declaration.
fn hand_turn_to(runner: &mut GameRunner, player: PlayerId) {
    runner.state_mut().active_player = player;
    runner.state_mut().priority_player = player;
    runner.state_mut().waiting_for = WaitingFor::Priority { player };
    for _ in 0..40 {
        if runner.waiting_for_kind() == "DeclareAttackers" {
            return;
        }
        runner.act(GameAction::PassPriority).expect("pass toward combat");
    }
    panic!("expected DeclareAttackers");
}

fn truce_zone_after_attack(attacker: PlayerId, target: impl Fn(&[ObjectId]) -> AttackTarget) -> Zone {
    let (mut runner, truce, creatures, walkers) = table();
    hand_turn_to(&mut runner, attacker);
    runner
        .declare_attackers(&[(creatures[attacker.0 as usize], target(&walkers))])
        .expect("the attack is legal");
    runner.advance_until_stack_empty();
    runner.state().objects[&truce].zone
}

#[test]
fn tenuous_truce_is_sacrificed_only_for_attacks_between_us_and_the_enchanted_opponent() {
    let player = |p: PlayerId| move |_: &[ObjectId]| AttackTarget::Player(p);
    let walker_of = |p: PlayerId| move |w: &[ObjectId]| AttackTarget::Planeswalker(w[p.0 as usize]);
    // We attack the enchanted opponent or their planeswalker.
    assert_eq!(truce_zone_after_attack(P0, player(P2)), Zone::Graveyard);
    assert_eq!(truce_zone_after_attack(P0, walker_of(P2)), Zone::Graveyard);
    // The enchanted opponent attacks us or our planeswalker.
    assert_eq!(truce_zone_after_attack(P2, player(P0)), Zone::Graveyard);
    assert_eq!(truce_zone_after_attack(P2, walker_of(P0)), Zone::Graveyard);
    // Attacks between other pairs leave it.
    assert_eq!(truce_zone_after_attack(P0, player(P1)), Zone::Battlefield);
    assert_eq!(truce_zone_after_attack(P0, walker_of(P1)), Zone::Battlefield);
    assert_eq!(truce_zone_after_attack(P2, player(P1)), Zone::Battlefield);
    assert_eq!(truce_zone_after_attack(P2, walker_of(P3)), Zone::Battlefield);
    assert_eq!(truce_zone_after_attack(P3, player(P0)), Zone::Battlefield);
    assert_eq!(truce_zone_after_attack(P1, walker_of(P0)), Zone::Battlefield);
}

/// Each player's end step in turn: only P2's draws, one card for P0 and one for P2.
#[test]
fn tenuous_truce_draws_only_at_the_enchanted_opponents_end_step() {
    for active in [P0, P1, P2, P3] {
        let (mut runner, truce, _, _) = table();
        hand_turn_to(&mut runner, active);
        runner.declare_attackers(&[]).expect("no attack");
        let hands: Vec<usize> = runner.state().players.iter().map(|p| p.hand.len()).collect();
        for _ in 0..80 {
            if runner.state().phase == Phase::End
                && matches!(runner.state().waiting_for, WaitingFor::Priority { .. })
            {
                break;
            }
            runner.act(GameAction::PassPriority).expect("pass toward the end step");
        }
        assert_eq!(runner.state().phase, Phase::End);
        let triggered = runner.state().stack.iter().filter(|e| e.source_id == truce).count();
        runner.advance_until_stack_empty();
        let drawn: Vec<usize> = runner
            .state()
            .players
            .iter()
            .zip(&hands)
            .map(|(p, before)| p.hand.len() - before)
            .collect();
        if active == P2 {
            assert_eq!(triggered, 1, "the enchanted opponent's end step triggers");
            assert_eq!(drawn, vec![1, 0, 1, 0], "we and the enchanted opponent draw");
        } else {
            assert_eq!(triggered, 0, "{active:?}'s end step does not trigger");
            assert_eq!(drawn, vec![0, 0, 0, 0], "nobody draws at {active:?}'s end step");
        }
    }
}
