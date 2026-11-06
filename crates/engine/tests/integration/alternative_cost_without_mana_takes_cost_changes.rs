//! An alternative cost with no mana part still takes cost increases.
//!
//! CR 118.9d: increases and reductions that affect a spell apply to the
//! alternative cost being paid. CR 601.2f: the total cost is that alternative
//! cost plus increases, minus reductions, and a mana component reduced to
//! nothing is {0}. Force of Will's "pay 1 life and exile a blue card from your
//! hand" has no mana part, so its total is {0} plus whatever increases apply;
//! its ruling says the same. The engine paid such a cost with no mana at all.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::actions::AlternativeCastDecision;
use engine::types::card_type::CoreType;
use engine::types::game_state::{CastingVariant, StackEntry, StackEntryKind};
use engine::types::identifiers::{CardId, ObjectId};
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

// Verbatim Oracle text (Scryfall).
const FORCE_OF_WILL: &str = "You may pay 1 life and exile a blue card from your hand rather than pay this spell's mana cost.\n\
Counter target spell.";
const GRAND_ARBITER: &str = "White spells you cast cost {1} less to cast.\n\
Blue spells you cast cost {1} less to cast.\n\
Spells your opponents cast cost {1} more to cast.";
const TRINISPHERE: &str = "As long as this artifact is untapped, each spell that would cost less than three mana to cast costs three mana to cast. (Additional mana in the cost may be paid with any color of mana or colorless mana. For example, a spell that would cost {1}{B} to cast costs {2}{B} to cast instead.)";
const OMNISCIENCE: &str = "You may cast spells from your hand without paying their mana costs.";

struct Board {
    runner: GameRunner,
    force: ObjectId,
    pitch: ObjectId,
    islands: Vec<ObjectId>,
    target: ObjectId,
}

/// Force of Will and a blue card in P0's hand, `islands` Islands, an opponent's
/// spell on the stack. `opponent_static` puts a permanent with that Oracle text
/// on P1's battlefield.
fn force_board(islands: usize, opponent_static: Option<(&str, &str)>) -> Board {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    if let Some((name, text)) = opponent_static {
        scenario.add_artifact_from_oracle(P1, name, text);
    }
    let islands = (0..islands)
        .map(|_| scenario.add_basic_land(P0, ManaColor::Blue))
        .collect();
    let force = scenario
        .add_spell_to_hand_from_oracle(P0, "Force of Will", true, FORCE_OF_WILL)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Blue, ManaCostShard::Blue],
            generic: 3,
        })
        .id();
    let pitch = scenario
        .add_creature_to_hand(P0, "Blue Pitch Card", 2, 2)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Blue],
            generic: 1,
        })
        .id();
    let mut runner = scenario.build();
    {
        let obj = runner.state_mut().objects.get_mut(&pitch).unwrap();
        obj.color.push(ManaColor::Blue);
        obj.base_color.push(ManaColor::Blue);
        let obj = runner.state_mut().objects.get_mut(&force).unwrap();
        obj.color.push(ManaColor::Blue);
        obj.base_color.push(ManaColor::Blue);
    }
    let target = engine::game::zones::create_object(
        runner.state_mut(),
        CardId(901),
        P1,
        "Opponent Sorcery".to_string(),
        Zone::Stack,
    );
    runner
        .state_mut()
        .objects
        .get_mut(&target)
        .unwrap()
        .card_types
        .core_types = vec![CoreType::Sorcery];
    runner.state_mut().stack.push_back(StackEntry {
        id: target,
        source_id: target,
        controller: P1,
        kind: StackEntryKind::Spell {
            card_id: CardId(901),
            ability: None,
            casting_variant: CastingVariant::Normal,
            actual_mana_spent: 0,
        },
    });
    Board {
        runner,
        force,
        pitch,
        islands,
        target,
    }
}

fn tapped(runner: &GameRunner, ids: &[ObjectId]) -> usize {
    ids.iter()
        .filter(|id| runner.state().objects[*id].tapped)
        .count()
}

fn cast_force_for_its_alternative_cost(b: &mut Board) -> Result<(), String> {
    let pitch = b.pitch;
    let target = b.target;
    b.runner
        .cast(b.force)
        .alternative_cast(AlternativeCastDecision::Alternative)
        .accept_optional()
        .target_objects(&[target])
        .pay_cost_with(&[pitch])
        .try_resolve()
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

/// Control: with no cost change, the alternative cost takes no mana.
#[test]
fn force_of_will_alternative_cost_takes_no_mana() {
    let mut b = force_board(2, None);
    cast_force_for_its_alternative_cost(&mut b).expect("the cast succeeds");
    assert_eq!(b.runner.state().objects[&b.target].zone, Zone::Graveyard);
    assert_eq!(b.runner.state().objects[&b.pitch].zone, Zone::Exile);
    assert_eq!(tapped(&b.runner, &b.islands), 0);
}

/// CR 118.9d + CR 601.2f: an opponent's Grand Arbiter Augustin IV adds {1} to
/// the {0} of the alternative cost: one Island of two is tapped.
#[test]
fn force_of_will_alternative_cost_takes_a_cost_increase() {
    let mut b = force_board(2, Some(("Grand Arbiter Augustin IV", GRAND_ARBITER)));
    cast_force_for_its_alternative_cost(&mut b).expect("the cast succeeds");
    assert_eq!(b.runner.state().objects[&b.target].zone, Zone::Graveyard);
    assert_eq!(b.runner.state().objects[&b.pitch].zone, Zone::Exile);
    assert_eq!(
        tapped(&b.runner, &b.islands),
        1,
        "the {{1}} increase is paid"
    );
}

/// CR 601.2h: without mana for the increase, the cast can't be completed.
#[test]
fn force_of_will_alternative_cost_needs_mana_for_the_increase() {
    let mut b = force_board(0, Some(("Grand Arbiter Augustin IV", GRAND_ARBITER)));
    let result = cast_force_for_its_alternative_cost(&mut b);
    assert!(
        result.is_err(),
        "the {{1}} increase can't be paid: {result:?}"
    );
    assert_ne!(b.runner.state().objects[&b.target].zone, Zone::Graveyard);
}

/// CR 601.2f: a cost floor applies after increases and reductions. Trinisphere
/// raises the {0} alternative cost to three mana.
#[test]
fn force_of_will_alternative_cost_takes_a_cost_floor() {
    let mut b = force_board(4, Some(("Trinisphere", TRINISPHERE)));
    cast_force_for_its_alternative_cost(&mut b).expect("the cast succeeds");
    assert_eq!(b.runner.state().objects[&b.target].zone, Zone::Graveyard);
    assert_eq!(
        tapped(&b.runner, &b.islands),
        3,
        "Trinisphere makes it three mana"
    );
}

/// CR 118.9 + CR 601.2f: casting "without paying its mana cost" is an
/// alternative cost too, so an opponent's Grand Arbiter Augustin IV makes a
/// spell cast through Omniscience cost {1}.
#[test]
fn free_cast_takes_a_cost_increase() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario
        .add_creature(P0, "Omniscience", 0, 0)
        .as_enchantment()
        .from_oracle_text(OMNISCIENCE);
    scenario.add_artifact_from_oracle(P1, "Grand Arbiter Augustin IV", GRAND_ARBITER);
    let forests: Vec<ObjectId> = (0..2)
        .map(|_| scenario.add_basic_land(P0, ManaColor::Green))
        .collect();
    let bear = scenario
        .add_creature_to_hand(P0, "Test Bear", 2, 2)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Green, ManaCostShard::Green],
            generic: 3,
        })
        .id();
    let mut runner = scenario.build();
    let outcome = runner.cast(bear).free_cast().resolve();
    assert_eq!(outcome.zone_of(bear), Zone::Battlefield);
    assert_eq!(tapped(&runner, &forests), 1, "the {{1}} increase is paid");
}

const NOURISHING_SHOAL: &str = "You may exile a green card with mana value X from your hand rather than pay this spell's mana cost.\n\
You gain X life.";

/// CR 118.9d + CR 107.3a: Nourishing Shoal's X still comes from the exiled
/// card under an opponent's Grand Arbiter Augustin IV, and the alternative
/// cost takes the {1}: X = 3, one Forest of two.
#[test]
fn shoal_defines_x_and_takes_a_cost_increase() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_artifact_from_oracle(P1, "Grand Arbiter Augustin IV", GRAND_ARBITER);
    let forests: Vec<ObjectId> = (0..2)
        .map(|_| scenario.add_basic_land(P0, ManaColor::Green))
        .collect();
    let shoal = scenario
        .add_spell_to_hand_from_oracle(P0, "Nourishing Shoal", true, NOURISHING_SHOAL)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::X, ManaCostShard::Green, ManaCostShard::Green],
            generic: 0,
        })
        .id();
    let pitch = scenario
        .add_creature_to_hand(P0, "Green Pitch Card", 3, 3)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Green],
            generic: 2,
        })
        .id();
    let mut runner = scenario.build();
    for id in [shoal, pitch] {
        let obj = runner.state_mut().objects.get_mut(&id).unwrap();
        obj.color.push(ManaColor::Green);
        obj.base_color.push(ManaColor::Green);
    }
    let life_before = runner.life(P0);
    runner
        .cast(shoal)
        .accept_optional()
        .pay_cost_with(&[pitch])
        .resolve();
    assert_eq!(runner.state().objects[&pitch].zone, Zone::Exile);
    assert_eq!(
        runner.life(P0),
        life_before + 3,
        "X is the exiled card's mana value"
    );
    assert_eq!(tapped(&runner, &forests), 1, "the {{1}} increase is paid");
}
