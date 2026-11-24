//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `mana_abilities.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::game::zones::create_object;
use crate::types::ability::{
    AbilityCost, AbilityKind, Effect, ManaContribution, ManaProduction, QuantityExpr,
};
use crate::types::card_type::CoreType;
use crate::types::counter::CounterType;
use crate::types::identifiers::CardId;
use crate::types::mana::{ManaColor, ManaType};
use crate::types::zones::Zone;
use std::sync::Arc;

#[test]
fn fixed_counter_removal_mana_cost_without_the_counter_is_refused() {
    let mut state = GameState::new_two_player(42);
    let player = PlayerId(0);
    let land = create_object(
        &mut state,
        CardId(8004),
        player,
        "Charge Land".to_string(),
        Zone::Battlefield,
    );
    let charge = CounterType::Generic("charge".to_string());
    {
        let obj = state.objects.get_mut(&land).unwrap();
        obj.card_types.core_types.push(CoreType::Land);
        obj.counters.insert(charge.clone(), 1);
        Arc::make_mut(&mut obj.abilities).push(
            AbilityDefinition::new(
                AbilityKind::Activated,
                Effect::Mana {
                    produced: ManaProduction::AnyOneColor {
                        count: QuantityExpr::Fixed { value: 1 },
                        color_options: vec![ManaColor::Green],
                        contribution: ManaContribution::Base,
                    },
                    restrictions: Vec::new(),
                    grants: Vec::new(),
                    expiry: None,
                    target: None,
                },
            )
            .cost(AbilityCost::Composite {
                costs: vec![
                    AbilityCost::Tap,
                    AbilityCost::RemoveCounter {
                        count: 1,
                        counter_type: CounterMatch::OfType(charge.clone()),
                        target: None,
                        selection: crate::types::ability::CounterCostSelection::SingleObject,
                    },
                ],
            }),
        );
    }
    let activate = |state: &mut GameState| {
        crate::game::engine::apply_as_current(
            state,
            crate::types::actions::GameAction::ActivateAbility {
                source_id: land,
                ability_index: 0,
                payment_mode: crate::types::game_state::CastPaymentMode::Auto,
            },
        )
    };

    activate(&mut state).expect("with its counter the ability is activated");
    assert_eq!(state.players[0].mana_pool.count_color(ManaType::Green), 1);
    assert_eq!(
        state.objects[&land]
            .counters
            .get(&charge)
            .copied()
            .unwrap_or(0),
        0
    );

    state.players[0].mana_pool = Default::default();
    state.objects.get_mut(&land).unwrap().tapped = false;
    assert!(
        activate(&mut state).is_err(),
        "without its counter the cost can't be paid in full"
    );
    assert_eq!(state.players[0].mana_pool.count_color(ManaType::Green), 0);
    assert!(
        !state.objects[&land].tapped,
        "the refused activation keeps no tap"
    );
}
