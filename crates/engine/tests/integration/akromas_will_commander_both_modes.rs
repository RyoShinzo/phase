//! Akroma's Will: "Choose one. If you control a commander as you cast this spell,
//! you may choose both instead." CR 700.2 + CR 903.3d: with a commander on the
//! caster's battlefield both modes may be chosen and both apply; without one
//! only one mode may be chosen.

use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::keywords::Keyword;
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;

use super::rules::cast_spell_action;

/// Verbatim Oracle text (Scryfall).
const AKROMAS_WILL: &str = "Choose one. If you control a commander as you cast this spell, you may choose both instead.\n\
    • Creatures you control gain flying, vigilance, and double strike until end of turn.\n\
    • Creatures you control gain lifelink, indestructible, and protection from each color until end of turn.";

/// A Knight, a creature that is a commander only if `with_commander`, and
/// Akroma's Will in hand at no cost. Returns the runner, the Knight and the spell.
fn board(with_commander: bool) -> (GameRunner, ObjectId, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let knight = scenario.add_creature(P0, "Knight", 2, 2).id();
    let mut leader = scenario.add_creature(P0, "Sidar Jabari of Zhalfir", 4, 3);
    if with_commander {
        leader.commander();
    }
    let will = scenario
        .add_spell_to_hand_from_oracle(P0, "Akroma's Will", true, AKROMAS_WILL)
        .with_mana_cost(ManaCost::generic(0))
        .id();
    (scenario.build(), knight, will)
}

fn cast_to_mode_choice(runner: &mut GameRunner, will: ObjectId) -> usize {
    let cast = cast_spell_action(runner, will);
    runner.act(cast).expect("casting Akroma's Will is accepted");
    match &runner.state().waiting_for {
        WaitingFor::ModeChoice { modal, .. } => modal.max_choices,
        other => panic!("expected the mode prompt, got {other:?}"),
    }
}

#[test]
fn akromas_will_with_a_commander_chooses_both_modes() {
    let (mut runner, knight, will) = board(true);
    assert_eq!(cast_to_mode_choice(&mut runner, will), 2);
    runner
        .act(GameAction::SelectModes {
            indices: vec![0, 1],
        })
        .expect("both modes are accepted with a commander");
    runner.advance_until_stack_empty();
    let obj = &runner.state().objects[&knight];
    for keyword in [
        Keyword::Flying,
        Keyword::Vigilance,
        Keyword::DoubleStrike,
        Keyword::Lifelink,
        Keyword::Indestructible,
    ] {
        assert!(obj.has_keyword(&keyword), "the Knight has {keyword:?}");
    }
}

#[test]
fn akromas_will_without_a_commander_chooses_one_mode() {
    let (mut runner, knight, will) = board(false);
    assert_eq!(cast_to_mode_choice(&mut runner, will), 1);
    assert!(
        runner
            .act(GameAction::SelectModes {
                indices: vec![0, 1],
            })
            .is_err(),
        "both modes are refused without a commander"
    );
    runner
        .act(GameAction::SelectModes { indices: vec![0] })
        .expect("one mode is accepted");
    runner.advance_until_stack_empty();
    let obj = &runner.state().objects[&knight];
    assert!(obj.has_keyword(&Keyword::DoubleStrike));
    assert!(!obj.has_keyword(&Keyword::Lifelink));
}
