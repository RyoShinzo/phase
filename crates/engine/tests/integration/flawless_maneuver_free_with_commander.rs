//! Flawless Maneuver: "If you control a commander, you may cast this spell without
//! paying its mana cost." CR 118.9 + CR 903.3d: with a commander on the caster's
//! battlefield the spell is offered and cast with no mana; without one it needs
//! its mana cost {2}{W}.

use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::keywords::Keyword;
use engine::types::mana::{ManaCost, ManaCostShard};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

use super::rules::cast_spell_action;

/// Verbatim Oracle text (Scryfall).
const FLAWLESS_MANEUVER: &str = "If you control a commander, you may cast this spell without paying its mana cost.\n\
    Creatures you control gain indestructible until end of turn.";

/// No mana anywhere, a Knight, a creature that is a commander only if
/// `with_commander`, and Flawless Maneuver ({2}{W}) in hand.
fn board(with_commander: bool) -> (GameRunner, ObjectId, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let knight = scenario.add_creature(P0, "Knight", 2, 2).id();
    let mut leader = scenario.add_creature(P0, "Sidar Jabari of Zhalfir", 4, 3);
    if with_commander {
        leader.commander();
    }
    let maneuver = scenario
        .add_spell_to_hand_from_oracle(P0, "Flawless Maneuver", true, FLAWLESS_MANEUVER)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::White],
            generic: 2,
        })
        .id();
    (scenario.build(), knight, maneuver)
}

fn offered(runner: &GameRunner, id: ObjectId) -> bool {
    engine::ai_support::legal_actions(runner.state())
        .into_iter()
        .any(|action| matches!(action, GameAction::CastSpell { object_id, .. } if object_id == id))
}

#[test]
fn flawless_maneuver_is_free_with_a_commander() {
    let (mut runner, knight, maneuver) = board(true);
    assert!(offered(&runner, maneuver), "the free cast is offered with no mana");
    let cast = cast_spell_action(&runner, maneuver);
    runner.act(cast).expect("the cast is accepted");
    // CR 118.9: the engine asks whether to use the alternative cost (no mana)
    // or the printed {2}{W}; only the free one is payable here.
    assert!(matches!(
        runner.state().waiting_for,
        WaitingFor::OptionalCostChoice { .. }
    ));
    runner
        .act(GameAction::DecideOptionalCost { pay: true })
        .expect("casting it without paying its mana cost is accepted");
    runner.advance_until_stack_empty();
    assert_eq!(runner.state().objects[&maneuver].zone, Zone::Graveyard);
    assert!(runner.state().objects[&knight].has_keyword(&Keyword::Indestructible));
}

#[test]
fn flawless_maneuver_costs_mana_without_a_commander() {
    let (mut runner, knight, maneuver) = board(false);
    assert!(!offered(&runner, maneuver), "no cast is offered without mana or a commander");
    let cast = cast_spell_action(&runner, maneuver);
    let _ = runner.act(cast);
    runner.advance_until_stack_empty();
    assert_eq!(runner.state().objects[&maneuver].zone, Zone::Hand, "it stays in hand");
    assert!(!runner.state().objects[&knight].has_keyword(&Keyword::Indestructible));
}
