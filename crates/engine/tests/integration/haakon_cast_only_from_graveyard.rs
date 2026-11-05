//! CR 601.3: Haakon, Stromgald Scourge — "You may cast this card from your
//! graveyard, but not from anywhere else." Its ruling: it can't be cast from the
//! hand, nor from any zone other than the graveyard even if an effect would allow
//! it. Its second ability still lets Knight spells be cast from the graveyard
//! while Haakon is on the battlefield.

use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::actions::GameAction;
use engine::types::game_state::CastPaymentMode;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

/// Verbatim Oracle text (Scryfall).
const HAAKON: &str = "You may cast this card from your graveyard, but not from anywhere else.\n\
    As long as Haakon is on the battlefield, you may cast Knight spells from your graveyard.\n\
    When Haakon dies, you lose 2 life.";

fn offered_cast(runner: &GameRunner, id: ObjectId) -> bool {
    engine::ai_support::legal_actions(runner.state())
        .into_iter()
        .any(|action| matches!(action, GameAction::CastSpell { object_id, .. } if object_id == id))
}

fn cast(runner: &mut GameRunner, id: ObjectId) -> bool {
    let card_id = runner.state().objects[&id].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: id,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .is_ok()
}

#[test]
fn haakon_cannot_be_cast_from_hand() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let haakon = scenario
        .add_creature_to_hand(P0, "Haakon, Stromgald Scourge", 3, 3)
        .with_subtypes(vec!["Zombie", "Knight"])
        .from_oracle_text(HAAKON)
        .id();
    let mut runner = scenario.build();
    assert!(!offered_cast(&runner, haakon), "no cast from the hand is offered");
    assert!(!cast(&mut runner, haakon), "a cast from the hand is refused");
    assert_eq!(runner.state().objects[&haakon].zone, Zone::Hand);
}

#[test]
fn haakon_is_cast_from_the_graveyard() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let haakon = scenario
        .add_creature_to_graveyard(P0, "Haakon, Stromgald Scourge", 3, 3)
        .with_subtypes(vec!["Zombie", "Knight"])
        .from_oracle_text(HAAKON)
        .id();
    let mut runner = scenario.build();
    assert!(offered_cast(&runner, haakon), "the graveyard cast is offered");
    assert!(cast(&mut runner, haakon), "the graveyard cast is accepted");
    runner.advance_until_stack_empty();
    assert_eq!(runner.state().objects[&haakon].zone, Zone::Battlefield);
}

/// The second ability: with Haakon on the battlefield, a Knight card in the
/// graveyard can be cast, a non-Knight cannot.
#[test]
fn haakon_on_the_battlefield_lets_knights_be_cast_from_the_graveyard() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario
        .add_creature(P0, "Haakon, Stromgald Scourge", 3, 3)
        .with_subtypes(vec!["Zombie", "Knight"])
        .from_oracle_text(HAAKON);
    let knight = scenario
        .add_creature_to_graveyard(P0, "Graveyard Knight", 2, 2)
        .with_subtypes(vec!["Human", "Knight"])
        .id();
    let soldier = scenario
        .add_creature_to_graveyard(P0, "Graveyard Soldier", 2, 2)
        .with_subtypes(vec!["Human", "Soldier"])
        .id();
    let mut runner = scenario.build();
    assert!(!offered_cast(&runner, soldier), "a non-Knight stays in the graveyard");
    assert!(offered_cast(&runner, knight), "the Knight cast is offered");
    assert!(cast(&mut runner, knight), "the Knight cast is accepted");
    runner.advance_until_stack_empty();
    assert_eq!(runner.state().objects[&knight].zone, Zone::Battlefield);
}
