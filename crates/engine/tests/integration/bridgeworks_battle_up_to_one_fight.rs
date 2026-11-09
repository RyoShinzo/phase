//! "It fights up to one target creature you don't control" may name no creature to fight.
//!
//! Repro (Bridgeworks Battle, the spell face of Bridgeworks Battle // Tanglespan
//! Bridgeworks): "Target creature you control gets +2/+2 until end of turn. It fights up to
//! one target creature you don't control." The fight's slot is "up to one" (`multi_target`
//! with a minimum of zero), but the Fight branches of target collection and assignment read
//! only the ability-wide `optional_targeting`: skipping the slot was refused ("Cannot skip a
//! required target"), and with no creature to fight the spell could not be cast at all.
//!
//! CR 601.2c + CR 701.14a: zero targets is a legal choice for an "up to one" slot; the
//! creature we control still gets +2/+2 (the card's rulings).

use engine::ai_support::legal_actions;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::triggers::drain_order_triggers_with_identity;
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaCost};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const BRIDGEWORKS_BATTLE: &str = "Target creature you control gets +2/+2 until end of turn. It fights up to one target creature you don't control. (Each deals damage equal to its power to the other.)";

/// Our 2/2, three Forests and Bridgeworks Battle in hand; player 1 holds a 3/3 if `theirs`.
fn battle_game(theirs: bool) -> (GameRunner, ObjectId, Option<ObjectId>, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let mine = scenario.add_creature(P0, "Mine", 2, 2).id();
    let opposing = theirs.then(|| scenario.add_creature(P1, "Theirs", 3, 3).id());
    for _ in 0..3 {
        scenario.add_basic_land(P0, ManaColor::Green);
    }
    let battle = scenario
        .add_spell_to_hand_from_oracle(P0, "Bridgeworks Battle", false, BRIDGEWORKS_BATTLE)
        .id();
    (scenario.build(), mine, opposing, battle)
}

/// Casts the Battle on `mine`, fighting `fought` or skipping the slot, and resolves it.
fn cast_battle(
    runner: &mut GameRunner,
    battle: ObjectId,
    mine: ObjectId,
    fought: Option<ObjectId>,
) {
    let card_id = runner.state().objects[&battle].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: battle,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("Bridgeworks Battle must be castable");
    let mut chosen_mine = false;
    for _ in 0..40 {
        match runner.state().waiting_for.clone() {
            WaitingFor::TargetSelection { .. } if !chosen_mine => {
                chosen_mine = true;
                runner
                    .act(GameAction::ChooseTarget {
                        target: Some(TargetRef::Object(mine)),
                    })
                    .expect("our creature");
            }
            WaitingFor::TargetSelection { .. } => {
                runner
                    .act(GameAction::ChooseTarget {
                        target: fought.map(TargetRef::Object),
                    })
                    .expect("the fight's slot");
            }
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => return,
            WaitingFor::Priority { .. } | WaitingFor::ManaPayment { .. } => {
                runner.act(GameAction::PassPriority).expect("pass");
            }
            other => panic!("unexpected prompt: {other:?}"),
        }
    }
    panic!("Bridgeworks Battle never finished resolving");
}

#[test]
fn fights_the_creature_named() {
    let (mut runner, mine, theirs, battle) = battle_game(true);
    cast_battle(&mut runner, battle, mine, theirs);
    let theirs = theirs.expect("their creature");
    assert_eq!(runner.state().objects[&mine].power, Some(4));
    assert_eq!(
        runner.state().objects[&mine].damage_marked,
        3,
        "the 3/3 dealt 3"
    );
    assert_eq!(
        runner.state().objects[&theirs].zone,
        Zone::Graveyard,
        "4 damage kills the 3/3"
    );
}

#[test]
fn skipping_the_fight_still_pumps() {
    let (mut runner, mine, theirs, battle) = battle_game(true);
    cast_battle(&mut runner, battle, mine, None);
    let theirs = theirs.expect("their creature");
    assert_eq!(runner.state().objects[&mine].power, Some(4));
    assert_eq!(runner.state().objects[&mine].damage_marked, 0);
    assert_eq!(runner.state().objects[&theirs].damage_marked, 0);
    assert_eq!(runner.state().objects[&theirs].zone, Zone::Battlefield);
}

#[test]
fn castable_with_no_creature_to_fight() {
    let (mut runner, mine, _, battle) = battle_game(false);
    cast_battle(&mut runner, battle, mine, None);
    assert_eq!(runner.state().objects[&mine].power, Some(4));
    assert_eq!(runner.state().objects[&battle].zone, Zone::Graveyard);
}

const NOVEL_NUNCHAKU: &str = "When this Equipment enters, attach it to target creature you control. When you do, equipped creature fights up to one target creature an opponent controls. (Each deals damage equal to its power to the other.)\nEquipped creature gets +1/+1 and has trample.\nEquip {3}";

/// A trigger whose fight slot is skipped for want of a creature to fight opens a target prompt
/// with every slot already done. Its one legal action submits the targets chosen so far, so a
/// client that answers only from the engine's legal actions (an AI seat, a replay tool) gets on.
#[test]
fn completed_target_prompt_offers_its_answer() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let mine = scenario.add_creature(P0, "Mine", 2, 2).id();
    scenario.add_basic_land(P0, ManaColor::Green);
    let nunchaku = scenario
        .add_artifact_to_hand_from_oracle(P0, "Novel Nunchaku", NOVEL_NUNCHAKU)
        .with_subtypes(vec!["Equipment"])
        .with_mana_cost(ManaCost::Cost {
            generic: 1,
            shards: vec![],
        })
        .id();
    let mut runner = scenario.build();
    let card_id = runner.state().objects[&nunchaku].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: nunchaku,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("Novel Nunchaku must be castable");
    let mut completed_prompts = 0;
    for _ in 0..60 {
        match runner.state().waiting_for.clone() {
            WaitingFor::OrderTriggers { .. } => {
                drain_order_triggers_with_identity(runner.state_mut());
            }
            WaitingFor::TriggerTargetSelection {
                target_slots,
                selection,
                ..
            }
            | WaitingFor::TargetSelection {
                target_slots,
                selection,
                ..
            } => {
                let actions = legal_actions(runner.state());
                if selection.current_slot >= target_slots.len() {
                    completed_prompts += 1;
                    let chosen = selection.selected_slots.iter().flatten().cloned().collect();
                    assert_eq!(actions, vec![GameAction::SelectTargets { targets: chosen }]);
                }
                let action = actions
                    .into_iter()
                    .next()
                    .expect("a target prompt offers an action");
                runner.act(action).expect("the offered action");
            }
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => break,
            WaitingFor::Priority { .. } | WaitingFor::ManaPayment { .. } => {
                runner.act(GameAction::PassPriority).expect("pass");
            }
            other => panic!("unexpected prompt: {other:?}"),
        }
    }
    assert!(
        runner.state().stack.is_empty(),
        "Novel Nunchaku never finished resolving"
    );
    assert_eq!(
        completed_prompts, 1,
        "the reflexive fight opens one prompt with its slot skipped"
    );
    assert_eq!(
        runner.state().objects[&nunchaku].attached_to,
        Some(mine.into())
    );
    assert_eq!(runner.state().objects[&mine].damage_marked, 0);
}
