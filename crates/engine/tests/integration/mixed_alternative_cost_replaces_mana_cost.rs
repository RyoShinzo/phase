//! Alternative costs made of mana and other parts replace the mana cost.
//!
//! CR 118.9: an alternative cost is paid rather than the spell's mana cost.
//! CR 601.2f + CR 118.9d: the total cost is the alternative cost plus cost
//! increases, minus cost reductions. An alternative cost that is mana alone
//! always replaced the mana cost; one with another part (tap creatures, exile a
//! card) was paid on top of the full mana cost, so Sephara, Sky's Blade needed
//! {W} and {4}{W}{W}{W}, and with one Plains its alternative cost failed.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::identifiers::ObjectId;
use engine::types::keywords::Keyword;
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

// Verbatim Oracle text (Scryfall).
const SEPHARA: &str = "You may pay {W} and tap four untapped creatures you control with flying rather than pay this spell's mana cost.\n\
Flying, lifelink\n\
Other creatures you control with flying have indestructible. (Damage and effects that say \"destroy\" don't destroy them.)";
const STALWART_VALKYRIE: &str = "You may pay {1}{W} and exile a creature card from your graveyard rather than pay this spell's mana cost.\n\
Flying";
const ZAHID: &str = "You may pay {3}{U} and tap an untapped artifact you control rather than pay this spell's mana cost.\n\
Flying";
const FIELDMIST_BORDERPOST: &str = "You may pay {1} and return a basic land you control to its owner's hand rather than pay this spell's mana cost.\n\
This artifact enters tapped.\n\
{T}: Add {W} or {U}.";
const GRAND_ARBITER: &str = "White spells you cast cost {1} less to cast.\n\
Blue spells you cast cost {1} less to cast.\n\
Spells your opponents cast cost {1} more to cast.";

struct Board {
    runner: GameRunner,
    spell: ObjectId,
    flyers: Vec<ObjectId>,
    plains: Vec<ObjectId>,
}

/// Sephara ({4}{W}{W}{W}) in P0's hand, four untapped flyers and one creature
/// without flying on P0's battlefield, `plains` Plains. With `arbiter`, P1
/// controls Grand Arbiter Augustin IV.
fn sephara_board(plains: usize, arbiter: bool) -> Board {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let flyers = (0..4)
        .map(|i| {
            scenario
                .add_creature(P0, &format!("Faerie {i}"), 1, 1)
                .with_keyword(Keyword::Flying)
                .id()
        })
        .collect();
    scenario.add_creature(P0, "Grizzly Bears", 2, 2);
    if arbiter {
        scenario.add_creature_from_oracle(P1, "Grand Arbiter Augustin IV", 2, 3, GRAND_ARBITER);
    }
    let plains = (0..plains)
        .map(|_| scenario.add_basic_land(P0, ManaColor::White))
        .collect();
    let spell = scenario
        .add_creature_to_hand_from_oracle(P0, "Sephara, Sky's Blade", 7, 7, SEPHARA)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::White; 3],
            generic: 4,
        })
        .id();
    Board {
        runner: scenario.build(),
        spell,
        flyers,
        plains,
    }
}

fn tapped(runner: &GameRunner, ids: &[ObjectId]) -> usize {
    ids.iter()
        .filter(|id| runner.state().objects[*id].tapped)
        .count()
}

/// CR 118.9: with eight Plains, the alternative cost taps one Plains for {W}
/// and the four flyers, not the eight lands of {W} plus {4}{W}{W}{W}.
#[test]
fn sephara_alternative_cost_pays_w_instead_of_the_mana_cost() {
    let mut b = sephara_board(8, false);
    let outcome = b
        .runner
        .cast(b.spell)
        .accept_optional()
        .pay_cost_with(&b.flyers.clone())
        .resolve();
    assert_eq!(outcome.zone_of(b.spell), Zone::Battlefield);
    assert_eq!(tapped(&b.runner, &b.plains), 1, "only {{W}} is paid in mana");
    assert_eq!(tapped(&b.runner, &b.flyers), 4, "the four flyers are tapped");
}

/// CR 118.9 + CR 601.2h: one untapped Plains pays the alternative cost.
#[test]
fn sephara_alternative_cost_with_one_plains() {
    let mut b = sephara_board(1, false);
    let outcome = b
        .runner
        .cast(b.spell)
        .accept_optional()
        .pay_cost_with(&b.flyers.clone())
        .resolve();
    assert_eq!(outcome.zone_of(b.spell), Zone::Battlefield);
    assert_eq!(tapped(&b.runner, &b.plains), 1);
    assert_eq!(tapped(&b.runner, &b.flyers), 4);
}

/// CR 601.2f + CR 118.9d: Grand Arbiter Augustin IV's increase applies to the
/// alternative cost: {1}{W} and four flyers, two Plains of three.
#[test]
fn cost_increase_applies_to_the_mixed_alternative_cost() {
    let mut b = sephara_board(3, true);
    let outcome = b
        .runner
        .cast(b.spell)
        .accept_optional()
        .pay_cost_with(&b.flyers.clone())
        .resolve();
    assert_eq!(outcome.zone_of(b.spell), Zone::Battlefield);
    assert_eq!(tapped(&b.runner, &b.plains), 2, "{{W}} plus the {{1}} increase");
    assert_eq!(tapped(&b.runner, &b.flyers), 4);
}

/// CR 118.9: declining the alternative cost pays the printed {4}{W}{W}{W}
/// and taps no creature.
#[test]
fn declining_pays_the_printed_mana_cost() {
    let mut b = sephara_board(8, false);
    let outcome = b.runner.cast(b.spell).decline_optional().resolve();
    assert_eq!(outcome.zone_of(b.spell), Zone::Battlefield);
    assert_eq!(tapped(&b.runner, &b.plains), 7);
    assert_eq!(tapped(&b.runner, &b.flyers), 0);
}

/// CR 601.2f + CR 118.9d: Stalwart Valkyrie's {1}{W} and an exiled creature
/// card, reduced by our own Grand Arbiter Augustin IV ("White spells you cast
/// cost {1} less") to {W}: one Plains of two, and the printed {3}{W} is never
/// paid.
#[test]
fn stalwart_valkyrie_alternative_cost_with_a_reduction() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_creature_from_oracle(P0, "Grand Arbiter Augustin IV", 2, 3, GRAND_ARBITER);
    let plains: Vec<ObjectId> = (0..2)
        .map(|_| scenario.add_basic_land(P0, ManaColor::White))
        .collect();
    let dead = scenario
        .add_creature_to_graveyard(P0, "Dead Creature", 2, 2)
        .id();
    let valkyrie = scenario
        .add_creature_to_hand_from_oracle(P0, "Stalwart Valkyrie", 3, 2, STALWART_VALKYRIE)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::White],
            generic: 3,
        })
        .id();
    let mut runner = scenario.build();
    {
        let obj = runner.state_mut().objects.get_mut(&valkyrie).unwrap();
        obj.color.push(ManaColor::White);
        obj.base_color.push(ManaColor::White);
    }
    let outcome = runner
        .cast(valkyrie)
        .accept_optional()
        .pay_cost_with(&[dead])
        .resolve();
    assert_eq!(outcome.zone_of(valkyrie), Zone::Battlefield);
    assert_eq!(outcome.zone_of(dead), Zone::Exile);
    assert_eq!(tapped(&runner, &plains), 1, "{{1}}{{W}} minus {{1}}");
}

/// CR 118.9: Zahid, Djinn of the Lamp ({4}{U}{U}) for {3}{U} and a tapped
/// artifact: four Islands of six.
#[test]
fn zahid_alternative_cost_pays_its_mana_part_only() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let islands: Vec<ObjectId> = (0..6)
        .map(|_| scenario.add_basic_land(P0, ManaColor::Blue))
        .collect();
    let relic = scenario.add_artifact_from_oracle(P0, "Relic", "").id();
    let zahid = scenario
        .add_creature_to_hand_from_oracle(P0, "Zahid, Djinn of the Lamp", 5, 6, ZAHID)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Blue; 2],
            generic: 4,
        })
        .id();
    let mut runner = scenario.build();
    let outcome = runner
        .cast(zahid)
        .accept_optional()
        .pay_cost_with(&[relic])
        .resolve();
    assert_eq!(outcome.zone_of(zahid), Zone::Battlefield);
    assert!(runner.state().objects[&relic].tapped, "the artifact is tapped");
    assert_eq!(tapped(&runner, &islands), 4, "{{3}}{{U}}, not {{3}}{{U}} plus {{4}}{{U}}{{U}}");
}

/// CR 118.9: Fieldmist Borderpost ({1}{W}{U}) for {1} and a returned basic
/// land: one land returns to hand and the other pays {1}.
#[test]
fn borderpost_alternative_cost_pays_one_and_returns_a_land() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let plains = scenario.add_basic_land(P0, ManaColor::White);
    let island = scenario.add_basic_land(P0, ManaColor::Blue);
    let post = scenario
        .add_artifact_to_hand_from_oracle(P0, "Fieldmist Borderpost", FIELDMIST_BORDERPOST)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::White, ManaCostShard::Blue],
            generic: 1,
        })
        .id();
    let mut runner = scenario.build();
    let outcome = runner
        .cast(post)
        .accept_optional()
        .pay_cost_with(&[plains])
        .resolve();
    assert_eq!(outcome.zone_of(post), Zone::Battlefield);
    assert_eq!(outcome.zone_of(plains), Zone::Hand);
    assert!(runner.state().objects[&island].tapped, "the Island pays {{1}}");
}
