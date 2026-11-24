//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `casting_tests.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::super::engine::apply_as_current;
use super::*;
use crate::game::zones::create_object;
use crate::types::ability::{QuantityExpr, TargetFilter};
use crate::types::actions::GameAction;
use crate::types::phase::Phase;
use std::sync::Arc;

fn setup_game_at_main_phase() -> GameState {
    let mut state = GameState::new_two_player(42);
    state.turn_number = 2;
    state.phase = Phase::PreCombatMain;
    state.active_player = PlayerId(0);
    state.priority_player = PlayerId(0);
    state.waiting_for = WaitingFor::Priority {
        player: PlayerId(0),
    };
    state
}

/// CR 702.26b: a phased-out permanent is treated as though it does not exist,
/// so its activated abilities are neither offered nor accepted, and are again
/// once it phases in.
#[test]
fn phased_out_permanent_activated_ability_is_not_offered_or_accepted() {
    let mut state = setup_game_at_main_phase();
    let source = create_object(
        &mut state,
        CardId(9_006),
        PlayerId(0),
        "Phasing Permanent".to_string(),
        Zone::Battlefield,
    );
    Arc::make_mut(&mut state.objects.get_mut(&source).unwrap().abilities).push(
        AbilityDefinition::new(
            AbilityKind::Activated,
            Effect::GainLife {
                amount: QuantityExpr::Fixed { value: 1 },
                player: TargetFilter::Controller,
            },
        ),
    );
    assert!(can_activate_ability_now(&state, PlayerId(0), source, 0));

    let mut events = Vec::new();
    crate::game::phasing::phase_out_object(
        &mut state,
        source,
        crate::game::game_object::PhaseOutCause::Directly,
        &mut events,
    );
    assert!(
        !can_activate_ability_now(&state, PlayerId(0), source, 0),
        "a phased-out permanent's ability must fail the legality gate (CR 702.26b)"
    );
    let principal = super::super::engine::priority_principal_for_preflight(&state)
        .expect("the synchronized priority window has a principal");
    assert!(
        priority_activate_ability_announcements(&state, &principal).is_empty(),
        "a phased-out permanent's ability must not be offered (CR 702.26b)"
    );
    let rejected = apply_as_current(
        &mut state,
        GameAction::ActivateAbility {
            source_id: source,
            ability_index: 0,
            payment_mode: CastPaymentMode::Auto,
        },
    );
    assert!(
        matches!(rejected, Err(EngineError::ActionNotAllowed(_))),
        "a phased-out permanent's ability must be rejected (CR 702.26b), got {rejected:?}"
    );

    state.objects.get_mut(&source).unwrap().phase_status =
        crate::game::game_object::PhaseStatus::PhasedIn;
    assert!(can_activate_ability_now(&state, PlayerId(0), source, 0));
}
