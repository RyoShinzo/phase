//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `choose_objects_into_tracked_set.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use crate::game::scenario::GameScenario;
use crate::types::ability::{
    AbilityDefinition, AbilityKind, Effect, TargetFilter, TargetRef, TypedFilter,
};
use crate::types::actions::GameAction;
use crate::types::game_state::WaitingFor;
use crate::types::phase::Phase;
use crate::types::player::PlayerId;

const P0: PlayerId = PlayerId(0);

const P1: PlayerId = PlayerId(1);

/// CR 608.2c + CR 608.2d: "choose any number of creatures with different
/// powers" — the prompt carries the set relation, the submission handler
/// refuses a set with two equal powers, and the AI is offered only sets
/// that hold it.
#[test]
fn set_relation_refuses_a_shared_power() {
    use crate::types::ability::{SearchSelectionConstraint, SharedQuality, SharedQualityRelation};
    let constraint = SearchSelectionConstraint::SharesQualities {
        qualities: vec![SharedQuality::Power],
        relation: SharedQualityRelation::DoesNotShare,
    };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let choice = AbilityDefinition::new(
        AbilityKind::Activated,
        Effect::ChooseObjectsIntoTrackedSet {
            chooser: TargetFilter::Controller,
            filter: TargetFilter::Typed(TypedFilter::creature()),
            min: 0,
            max: None,
            cardinality: None,
            eligibility: None,
            selection_constraint: constraint.clone(),
        },
    );
    let host = {
        let mut builder = scenario.add_artifact_from_oracle(P0, "Relation Host", "");
        builder.with_ability_definition(choice);
        builder.id()
    };
    let one = scenario.add_creature(P0, "One", 1, 1).id();
    let two_a = scenario.add_creature(P0, "Two A", 2, 2).id();
    let two_b = scenario.add_creature(P1, "Two B", 2, 2).id();

    let mut runner = scenario.build();
    runner
        .act(GameAction::ActivateAbility {
            source_id: host,
            ability_index: 0,
            payment_mode: crate::types::game_state::CastPaymentMode::Auto,
        })
        .expect("activate the choice");
    runner.advance_until_stack_empty();
    let WaitingFor::ChooseObjectsSelection {
        selection_constraint,
        ..
    } = &runner.state().waiting_for
    else {
        panic!(
            "expected the object selection, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(*selection_constraint, constraint);

    let offered = crate::ai_support::legal_actions(runner.state());
    let selects: Vec<&Vec<TargetRef>> = offered
        .iter()
        .filter_map(|action| match action {
            GameAction::SelectTargets { targets } => Some(targets),
            _ => None,
        })
        .collect();
    assert!(!selects.is_empty());
    assert!(
        selects.iter().all(|targets| {
            !(targets.contains(&TargetRef::Object(two_a))
                && targets.contains(&TargetRef::Object(two_b)))
        }),
        "no offered set holds both power-2 creatures: {selects:?}"
    );

    assert!(runner
        .act(GameAction::SelectTargets {
            targets: vec![
                TargetRef::Object(one),
                TargetRef::Object(two_a),
                TargetRef::Object(two_b),
            ],
        })
        .is_err());
    runner
        .act(GameAction::SelectTargets {
            targets: vec![TargetRef::Object(one), TargetRef::Object(two_b)],
        })
        .expect("different powers, either player's creatures");
}
