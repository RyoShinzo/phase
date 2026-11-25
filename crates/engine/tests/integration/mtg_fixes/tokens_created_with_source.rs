//! CR 607.2c + CR 400.7: "tokens created with [this object]" (Faerie Artisans,
//! Saproling Burst, Arcane Artisan, Tetravus) are the tokens an ability of that
//! exact object created. Each token records the object it was created with, so
//! another source's tokens, and tokens of an earlier object that had the same
//! storage id, are not among them; a source that left before its ability
//! resolved still created the token (Arcane Artisan's ruling).

use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::game::zones::move_to_zone;
use engine::types::actions::GameAction;
use engine::types::game_state::CastPaymentMode;
use engine::types::identifiers::{ObjectId, ObjectIncarnationRef};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const MAKER: &str = "{T}: Create a 1/1 white Soldier creature token.\n\
    {0}: Exile all tokens created with this creature.";
const LEAVER: &str = "{T}: Create a 1/1 white Soldier creature token.\n\
    When this creature leaves the battlefield, exile all tokens created with it.\n\
    Sacrifice this creature: You gain 1 life.";

fn tokens_on_battlefield(runner: &GameRunner) -> Vec<ObjectId> {
    let state = runner.state();
    state
        .battlefield
        .iter()
        .copied()
        .filter(|id| state.objects[id].is_token)
        .collect()
}

/// The token `source`'s first ability creates.
fn create_token(runner: &mut GameRunner, source: ObjectId) -> ObjectId {
    let before = tokens_on_battlefield(runner);
    runner.activate(source, 0).resolve();
    runner.advance_until_stack_empty();
    let created: Vec<ObjectId> = tokens_on_battlefield(runner)
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(created.len(), 1, "one token created");
    created[0]
}

/// Two sources with the same abilities and a third token maker: the exile
/// ability of one takes only the token it created (Faerie Artisans' ruling:
/// each one's ability exiles only the tokens created with it).
#[test]
fn each_source_exiles_only_the_tokens_it_created() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let first = scenario.add_creature_from_oracle(P0, "First Maker", 1, 1, MAKER).id();
    let second = scenario.add_creature_from_oracle(P0, "Second Maker", 1, 1, MAKER).id();
    let other = scenario.add_creature_from_oracle(P0, "Other Maker", 1, 1, LEAVER).id();
    let mut runner = scenario.build();
    let firsts = create_token(&mut runner, first);
    let seconds = create_token(&mut runner, second);
    let others = create_token(&mut runner, other);
    assert_eq!(
        runner.state().objects[&firsts].created_with,
        Some(ObjectIncarnationRef::of(first, runner.state().objects[&first].incarnation))
    );

    runner.activate(first, 1).resolve();
    runner.advance_until_stack_empty();
    let left = tokens_on_battlefield(&runner);
    assert!(!left.contains(&firsts), "the first maker's token is exiled");
    assert!(left.contains(&seconds), "the second maker's token stays");
    assert!(left.contains(&others), "the other maker's token stays");
}

/// CR 603.10a: the leaves-the-battlefield trigger names the object that left,
/// which created the token (Saproling Burst, Arcane Artisan).
#[test]
fn leaving_the_battlefield_exiles_the_tokens_it_created() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let leaver = scenario.add_creature_from_oracle(P0, "Leaving Maker", 1, 1, LEAVER).id();
    let other = scenario.add_creature_from_oracle(P0, "Other Maker", 1, 1, MAKER).id();
    let mut runner = scenario.build();
    let leavers = create_token(&mut runner, leaver);
    let others = create_token(&mut runner, other);

    runner.activate(leaver, 1).resolve();
    runner.advance_until_stack_empty();
    assert_eq!(runner.state().objects[&leaver].zone, Zone::Graveyard);
    let left = tokens_on_battlefield(&runner);
    assert!(!left.contains(&leavers), "the leaving maker's token is exiled");
    assert!(left.contains(&others), "the other maker's token stays");
}

/// CR 400.7: a source that left and returned is a new object, which created
/// none of the tokens its earlier self did.
#[test]
fn the_returned_object_created_none_of_them() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let maker = scenario.add_creature_from_oracle(P0, "Returning Maker", 1, 1, MAKER).id();
    let mut runner = scenario.build();
    let token = create_token(&mut runner, maker);

    let mut events = Vec::new();
    move_to_zone(runner.state_mut(), maker, Zone::Exile, &mut events);
    move_to_zone(runner.state_mut(), maker, Zone::Battlefield, &mut events);
    runner.advance_until_stack_empty();
    runner.activate(maker, 1).resolve();
    runner.advance_until_stack_empty();
    assert!(tokens_on_battlefield(&runner).contains(&token), "the earlier object's token stays");
}

/// CR 607.2c + CR 113.7a: a source that left the battlefield before its
/// ability resolved still created the token, as the object it was when the
/// ability was activated (Arcane Artisan's ruling).
#[test]
fn a_source_that_left_before_resolution_still_created_the_token() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let maker = scenario.add_creature_from_oracle(P0, "Departing Maker", 1, 1, MAKER).id();
    let mut runner = scenario.build();
    let activated_as = ObjectIncarnationRef::of(maker, runner.state().objects[&maker].incarnation);
    runner
        .act(GameAction::ActivateAbility {
            source_id: maker,
            ability_index: 0,
            payment_mode: CastPaymentMode::default(),
        })
        .expect("activate the token ability");
    assert_eq!(runner.state().stack.len(), 1, "the ability waits on the stack");

    let mut events = Vec::new();
    move_to_zone(runner.state_mut(), maker, Zone::Graveyard, &mut events);
    runner.advance_until_stack_empty();
    let tokens = tokens_on_battlefield(&runner);
    assert_eq!(tokens.len(), 1, "the token is still created");
    assert_eq!(runner.state().objects[&tokens[0]].created_with, Some(activated_as));
}
