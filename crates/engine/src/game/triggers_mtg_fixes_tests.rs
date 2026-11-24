//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `triggers.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::game::zones::create_object;
use crate::types::ability::{Comparator, QuantityExpr, QuantityRef, TriggerCondition};
use crate::types::events::GameEvent;
use crate::types::game_state::GameState;
use crate::types::identifiers::{CardId, ObjectId};
use crate::types::mana::ManaColor;
use crate::types::player::PlayerId;
use crate::types::zones::Zone;

fn setup() -> GameState {
    GameState::new_two_player(42)
}

#[test]
fn card_drawn_cause_matches_only_draws() {
    // CR 603.2d + CR 121.1: Krang doubles a trigger a draw caused, not one
    // caused by attacking or by no event.
    let state = setup();
    let draw = GameEvent::CardDrawn {
        player_id: PlayerId(1),
        object_id: ObjectId(1),
        nth_in_turn: 2,
        nth_in_step: 1,
    };
    let attack = GameEvent::AttackersDeclared {
        attacker_ids: vec![],
        defending_player: PlayerId(1),
        attacks: vec![],
        declaration_records: vec![],
    };
    assert!(trigger_cause_matches(
        &state,
        &TriggerCause::CardDrawn,
        Some(&draw),
        PlayerId(0)
    ));
    assert!(!trigger_cause_matches(
        &state,
        &TriggerCause::CardDrawn,
        Some(&attack),
        PlayerId(0)
    ));
    assert!(!trigger_cause_matches(
        &state,
        &TriggerCause::CardDrawn,
        None,
        PlayerId(0)
    ));
}

#[test]
fn controller_gained_life_cause_matches_only_own_gains() {
    // CR 603.2d + CR 119.3: Dr. Beverly Crusher doubles a trigger its
    // controller's gain caused, not another player's gain, a loss, or a
    // trigger caused by a creature entering.
    let state = setup();
    let gain = |player: u8, amount: i32| GameEvent::LifeChanged {
        player_id: PlayerId(player),
        amount,
        new_total: Default::default(),
    };
    let cause = TriggerCause::ControllerGainedLife;
    assert!(trigger_cause_matches(
        &state,
        &cause,
        Some(&gain(0, 3)),
        PlayerId(0)
    ));
    assert!(!trigger_cause_matches(
        &state,
        &cause,
        Some(&gain(1, 3)),
        PlayerId(0)
    ));
    assert!(!trigger_cause_matches(
        &state,
        &cause,
        Some(&gain(0, -2)),
        PlayerId(0)
    ));
    let draw = GameEvent::CardDrawn {
        player_id: PlayerId(0),
        object_id: ObjectId(1),
        nth_in_turn: 1,
        nth_in_step: 1,
    };
    assert!(!trigger_cause_matches(
        &state,
        &cause,
        Some(&draw),
        PlayerId(0)
    ));
    assert!(!trigger_cause_matches(&state, &cause, None, PlayerId(0)));
}

/// CR 106.1b + CR 601.2h + CR 603.4: "if {C} was spent to cast it" (Drowner
/// of Truth) reads the colorless part of the triggering spell's payment: the
/// amount spent less its five-color tally. Colored mana alone never satisfies
/// it, however much was spent.
#[test]
fn colorless_mana_spent_condition_reads_the_payment_less_its_colors() {
    let mut state = setup();
    let spell = create_object(
        &mut state,
        CardId(2),
        PlayerId(0),
        "Drowner of Truth".to_string(),
        Zone::Stack,
    );
    let condition = TriggerCondition::QuantityComparison {
        lhs: QuantityExpr::Ref {
            qty: QuantityRef::ManaSpentToCast {
                scope: crate::types::ability::CastManaObjectScope::TriggeringSpell,
                metric: crate::types::ability::CastManaSpentMetric::Colorless,
            },
        },
        comparator: Comparator::GE,
        rhs: QuantityExpr::Fixed { value: 1 },
    };
    let event = GameEvent::SpellCast {
        card_id: CardId(2),
        controller: PlayerId(0),
        object_id: spell,
        cast_mana_value: None,
    };
    let pay = |state: &mut GameState, colored: &[ManaColor], total: u32| {
        let obj = state.objects.get_mut(&spell).unwrap();
        obj.colors_spent_to_cast = crate::types::mana::ColoredManaCount::default();
        for color in colored {
            obj.colors_spent_to_cast.add(*color, 1);
        }
        obj.mana_spent_to_cast_amount = total;
    };

    // Seven mana, all green and blue: no colorless.
    pay(
        &mut state,
        &[ManaColor::Green; 4]
            .iter()
            .chain(&[ManaColor::Blue; 3])
            .copied()
            .collect::<Vec<_>>(),
        7,
    );
    assert!(!check_trigger_condition(
        &state,
        &condition,
        PlayerId(0),
        Some(spell),
        Some(&event)
    ));

    // Five colorless and two green.
    pay(&mut state, &[ManaColor::Green, ManaColor::Green], 7);
    assert!(check_trigger_condition(
        &state,
        &condition,
        PlayerId(0),
        Some(spell),
        Some(&event)
    ));

    // Nothing spent.
    pay(&mut state, &[], 0);
    assert!(!check_trigger_condition(
        &state,
        &condition,
        PlayerId(0),
        Some(spell),
        Some(&event)
    ));
}
