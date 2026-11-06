//! A land returned to hand to pay a spell's cost can tap for that spell's mana
//! first.
//!
//! CR 601.2g: mana abilities are activated before costs are paid. CR 601.2h:
//! the costs are then paid in any order. Fieldmist Borderpost's ruling: "you
//! may tap a basic land for mana, then both spend that mana and return that
//! land to your hand to pay this card's alternative cost." The engine returned
//! the land before any mana was produced, so with a single basic land the
//! alternative cost could not be paid.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

// Verbatim Oracle text (Scryfall).
const FIELDMIST_BORDERPOST: &str = "You may pay {1} and return a basic land you control to its owner's hand rather than pay this spell's mana cost.\n\
This artifact enters tapped.\n\
{T}: Add {W} or {U}.";
const GRAND_ARBITER: &str = "White spells you cast cost {1} less to cast.\n\
Blue spells you cast cost {1} less to cast.\n\
Spells your opponents cast cost {1} more to cast.";

fn borderpost(scenario: &mut GameScenario) -> ObjectId {
    scenario
        .add_artifact_to_hand_from_oracle(P0, "Fieldmist Borderpost", FIELDMIST_BORDERPOST)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::White, ManaCostShard::Blue],
            generic: 1,
        })
        .id()
}

fn cast_returning(runner: &mut GameRunner, post: ObjectId, land: ObjectId) -> Zone {
    runner
        .cast(post)
        .accept_optional()
        .pay_cost_with(&[land])
        .resolve()
        .zone_of(post)
}

/// The ruling's case: the only basic land taps for {1}, then returns to hand.
#[test]
fn borderpost_with_one_basic_land() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let plains = scenario.add_basic_land(P0, ManaColor::White);
    let post = borderpost(&mut scenario);
    let mut runner = scenario.build();
    assert_eq!(cast_returning(&mut runner, post, plains), Zone::Battlefield);
    assert_eq!(runner.state().objects[&plains].zone, Zone::Hand);
}

/// With two lands, the returned one pays the {1}, so the other stays untapped.
#[test]
fn borderpost_taps_the_returned_land_first() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let plains = scenario.add_basic_land(P0, ManaColor::White);
    let island = scenario.add_basic_land(P0, ManaColor::Blue);
    let post = borderpost(&mut scenario);
    let mut runner = scenario.build();
    assert_eq!(cast_returning(&mut runner, post, plains), Zone::Battlefield);
    assert_eq!(runner.state().objects[&plains].zone, Zone::Hand);
    assert!(
        !runner.state().objects[&island].tapped,
        "the Island is not needed"
    );
}

/// CR 118.9d: under an opponent's Grand Arbiter Augustin IV the alternative
/// cost is {2}: the returned Plains pays one, the Island the other.
#[test]
fn borderpost_returned_land_pays_part_of_an_increased_cost() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_artifact_from_oracle(P1, "Grand Arbiter Augustin IV", GRAND_ARBITER);
    let plains = scenario.add_basic_land(P0, ManaColor::White);
    let island = scenario.add_basic_land(P0, ManaColor::Blue);
    let post = borderpost(&mut scenario);
    let mut runner = scenario.build();
    assert_eq!(cast_returning(&mut runner, post, plains), Zone::Battlefield);
    assert_eq!(runner.state().objects[&plains].zone, Zone::Hand);
    assert!(
        runner.state().objects[&island].tapped,
        "the Island pays the increase"
    );
}

/// CR 601.2h: one land can't pay {2}; the cast fails and nothing is returned.
#[test]
fn borderpost_with_one_land_under_an_increase_fails() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_artifact_from_oracle(P1, "Grand Arbiter Augustin IV", GRAND_ARBITER);
    let plains = scenario.add_basic_land(P0, ManaColor::White);
    let post = borderpost(&mut scenario);
    let mut runner = scenario.build();
    let result = runner
        .cast(post)
        .accept_optional()
        .pay_cost_with(&[plains])
        .try_resolve();
    assert!(result.is_err(), "{{2}} can't be paid with one land");
    assert_ne!(runner.state().objects[&post].zone, Zone::Battlefield);
}

const DEPRIVE: &str =
    "As an additional cost to cast this spell, return a land you control to its owner's hand.\n\
Counter target spell.";
const DAZE: &str = "You may return an Island you control to its owner's hand rather than pay this spell's mana cost.\n\
Counter target spell unless its controller pays {1}.";

/// An opponent's sorcery on the stack, for a counterspell to target.
fn opponent_spell(runner: &mut GameRunner) -> ObjectId {
    use engine::types::card_type::CoreType;
    use engine::types::game_state::{CastingVariant, StackEntry, StackEntryKind};
    use engine::types::identifiers::CardId;
    let spell = engine::game::zones::create_object(
        runner.state_mut(),
        CardId(902),
        P1,
        "Opponent Sorcery".to_string(),
        Zone::Stack,
    );
    runner
        .state_mut()
        .objects
        .get_mut(&spell)
        .unwrap()
        .card_types
        .core_types = vec![CoreType::Sorcery];
    runner.state_mut().stack.push_back(StackEntry {
        id: spell,
        source_id: spell,
        controller: P1,
        kind: StackEntryKind::Spell {
            card_id: CardId(902),
            ability: None,
            casting_variant: CastingVariant::Normal,
            actual_mana_spent: 0,
        },
    });
    spell
}

/// CR 601.2g + CR 601.2h, for an additional cost: Deprive ({U}{U}, return a
/// land) with exactly two Islands taps both and returns one of them.
#[test]
fn deprive_with_two_islands() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let islands: Vec<ObjectId> = (0..2)
        .map(|_| scenario.add_basic_land(P0, ManaColor::Blue))
        .collect();
    let deprive = scenario
        .add_spell_to_hand_from_oracle(P0, "Deprive", true, DEPRIVE)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Blue, ManaCostShard::Blue],
            generic: 0,
        })
        .id();
    let mut runner = scenario.build();
    let target = opponent_spell(&mut runner);
    runner
        .cast(deprive)
        .target_objects(&[target])
        .pay_cost_with(&[islands[0]])
        .resolve();
    assert_eq!(runner.state().objects[&target].zone, Zone::Graveyard);
    assert_eq!(runner.state().objects[&islands[0]].zone, Zone::Hand);
    assert!(runner.state().objects[&islands[1]].tapped);
}

/// CR 118.9d with both fixes: under an opponent's Grand Arbiter Augustin IV,
/// Daze's alternative cost is {1} and an Island; the one Island taps for the
/// {1} and then returns to hand.
#[test]
fn daze_with_one_island_under_an_increase() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_artifact_from_oracle(P1, "Grand Arbiter Augustin IV", GRAND_ARBITER);
    let island = scenario.add_basic_land(P0, ManaColor::Blue);
    let daze = scenario
        .add_spell_to_hand_from_oracle(P0, "Daze", true, DAZE)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Blue],
            generic: 1,
        })
        .id();
    let mut runner = scenario.build();
    let target = opponent_spell(&mut runner);
    runner
        .cast(daze)
        .accept_optional()
        .target_objects(&[target])
        .pay_cost_with(&[island])
        .commit();
    assert_eq!(runner.state().objects[&island].zone, Zone::Hand);
    let spent = runner
        .state()
        .stack
        .iter()
        .find(|entry| entry.id == daze)
        .map(|entry| match &entry.kind {
            engine::types::game_state::StackEntryKind::Spell {
                actual_mana_spent, ..
            } => *actual_mana_spent,
            _ => u32::MAX,
        });
    assert_eq!(spent, Some(1), "Daze is on the stack with {{1}} spent");
}
