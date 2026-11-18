//! Resplendent Marshal: "When this creature enters or dies, you may exile another creature card
//! from your graveyard." CR 400.7e: a dies trigger finds the card its source became in the
//! graveyard, and "another" excludes that card as it excludes the source on the battlefield.

use engine::game::scenario::{GameScenario, P0};
use engine::game::triggers::process_triggers;
use engine::game::zone_pipeline::{move_object_for_test, ZoneMoveRequest};
use engine::types::actions::GameAction;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const MARSHAL: &str = "When this creature enters or dies, you may exile another creature card from your graveyard. When you do, put a +1/+1 counter on each creature you control other than this creature that shares a creature type with the exiled card.";

#[test]
fn dies_trigger_another_creature_card_excludes_the_dying_source() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let marshal = scenario
        .add_creature_from_oracle(P0, "Resplendent Marshal", 3, 3, MARSHAL)
        .id();
    let other = scenario.add_creature_to_graveyard(P0, "Other", 2, 2).id();
    let mut runner = scenario.build();

    let mut events = Vec::new();
    assert!(!move_object_for_test(
        runner.state_mut(),
        ZoneMoveRequest::effect(marshal, Zone::Graveyard, marshal),
        &mut events,
    ));
    process_triggers(runner.state_mut(), &events);
    runner.advance_until_stack_empty();
    runner
        .act(GameAction::DecideOptionalEffect { accept: true })
        .expect("accept the optional exile");
    runner.advance_until_stack_empty();
    // Only the other creature card is legal, so it is exiled without a choice.
    assert_eq!(runner.state().objects[&other].zone, Zone::Exile);
    assert_eq!(
        runner.state().objects[&marshal].zone,
        Zone::Graveyard,
        "the Marshal card is not another creature card"
    );
}
