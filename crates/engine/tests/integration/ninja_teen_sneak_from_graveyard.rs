//! A graveyard permission that requires sneak casts the creature from the graveyard.
//!
//! Repro (Ninja Teen, level 3): "Creature cards in your graveyard have sneak {3}{B}. You may
//! cast creature spells from your graveyard using their sneak abilities." The permission
//! was declined because the engine cast sneak only from the hand, so nothing could be cast
//! from the graveyard.
//!
//! CR 702.190a + CR 118.9b: in our declare blockers step the creature card is cast from the
//! graveyard for {3}{B}, returning an unblocked attacker to its owner's hand; it enters
//! tapped and attacking the player that attacker attacked (CR 702.190b). The permission is
//! the Class's level 3 ability, so at level 2 there is no such cast.

use engine::ai_support::legal_actions;
use engine::game::combat::{AttackerInfo, CombatState};
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaCost};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const NINJA_TEEN: &str = "Whenever a creature you control leaves the battlefield, each opponent loses 1 life.\n{1}{B}: Level 2\nCreatures you control get +1/+0 and have menace.\n{B}: Level 3\nCreature cards in your graveyard have sneak {3}{B}.\nYou may cast creature spells from your graveyard using their sneak abilities.";

/// Ninja Teen at `level`, an unblocked attacker at player 1 in our declare blockers step,
/// a 5/5 creature card costing {7} in our graveyard and four Swamps.
fn sneak_game(level: u8) -> (GameRunner, ObjectId, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::DeclareBlockers);
    let teen = scenario
        .add_enchantment_from_oracle(P0, "Ninja Teen", "")
        .with_subtypes(vec!["Class"])
        .from_oracle_text(NINJA_TEEN)
        .id();
    let attacker = scenario.add_creature(P0, "Attacker", 2, 2).id();
    let buried = scenario
        .add_creature_to_graveyard(P0, "Buried Giant", 5, 5)
        .with_mana_cost(ManaCost::generic(7))
        .id();
    for _ in 0..4 {
        scenario.add_basic_land(P0, ManaColor::Black);
    }
    let mut runner = scenario.build();
    let state = runner.state_mut();
    state.objects.get_mut(&teen).unwrap().class_level = Some(level);
    state.objects.get_mut(&attacker).unwrap().tapped = true;
    state.phase = Phase::DeclareBlockers;
    state.active_player = P0;
    state.priority_player = P0;
    state.waiting_for = WaitingFor::Priority { player: P0 };
    state.combat = Some(CombatState {
        attackers: vec![AttackerInfo::attacking_player(attacker, P1)],
        ..Default::default()
    });
    (runner, attacker, buried)
}

fn sneak_action(runner: &GameRunner, card: ObjectId) -> Option<GameAction> {
    legal_actions(runner.state()).into_iter().find(|action| {
        matches!(action, GameAction::CastSpellAsSneak { hand_object, .. } if *hand_object == card)
    })
}

#[test]
fn level_three_casts_a_creature_card_from_the_graveyard_with_sneak() {
    let (mut runner, attacker, buried) = sneak_game(3);
    assert_eq!(
        engine::game::keywords::effective_sneak_cost(runner.state(), buried),
        Some(ManaCost::Cost {
            shards: vec![engine::types::mana::ManaCostShard::Black],
            generic: 3,
        }),
        "reach: level 3 grants sneak {{3}}{{B}} to the graveyard card"
    );
    let action = sneak_action(&runner, buried).expect("the sneak cast from the graveyard");
    runner.act(action).expect("sneak cast");
    for _ in 0..20 {
        match runner.state().waiting_for.clone() {
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => break,
            WaitingFor::Priority { .. } | WaitingFor::ManaPayment { .. } => {
                runner.act(GameAction::PassPriority).expect("pass");
            }
            other => panic!("unexpected prompt: {other:?}"),
        }
    }
    let state = runner.state();
    assert_eq!(state.objects[&buried].zone, Zone::Battlefield);
    assert!(state.objects[&buried].tapped, "enters tapped (CR 702.190b)");
    let combat = state.combat.as_ref().expect("combat goes on");
    assert!(
        combat
            .attackers
            .iter()
            .any(|info| info.object_id == buried && info.defending_player == P1),
        "attacking the player the returned creature attacked"
    );
    assert_eq!(
        state.objects[&attacker].zone,
        Zone::Hand,
        "the attacker returned"
    );
    let swamps_tapped = state
        .battlefield
        .iter()
        .filter(|id| state.objects[id].name.contains("Swamp") && state.objects[id].tapped)
        .count();
    assert_eq!(swamps_tapped, 4, "{{3}}{{B}}, not the card's {{7}}");
}

#[test]
fn level_two_offers_no_graveyard_cast() {
    let (runner, _, buried) = sneak_game(2);
    assert!(sneak_action(&runner, buried).is_none());
    assert_eq!(runner.state().objects[&buried].zone, Zone::Graveyard);
}
