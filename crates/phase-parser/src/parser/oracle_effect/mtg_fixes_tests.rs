//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `tests.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::parser::parse_oracle_text;
use crate::types::ability::extract_target_filter_from_effect;
use crate::types::ability::{
    AbilityCondition, Comparator, ControllerRef, CopyRetargetPermission, Duration, FilterProp,
    QuantityExpr, QuantityRef, SearchSelectionConstraint, SharedQuality, TypeFilter, TypedFilter,
};
use crate::types::counter::CounterType;
use crate::types::zones::Zone;

fn assert_plus_counter_node(
    def: &AbilityDefinition,
    count: i32,
    another: bool,
) -> Option<&AbilityDefinition> {
    match def.effect.as_ref() {
        Effect::PutCounter {
            counter_type,
            count: QuantityExpr::Fixed { value },
            target,
        } => {
            assert_eq!(*counter_type, CounterType::Plus1Plus1);
            assert_eq!(*value, count);
            let tf = typed_leg(target).expect("counter target should be typed");
            assert!(has_type(tf, TypeFilter::Creature));
            assert_eq!(
                tf.properties.contains(&FilterProp::Another),
                another,
                "unexpected Another property on {tf:?}",
            );
        }
        other => panic!("expected PutCounter, got {other:?}"),
    }
    def.sub_ability.as_deref()
}

fn typed_leg(filter: &TargetFilter) -> Option<&TypedFilter> {
    match filter {
        TargetFilter::Typed(tf) => Some(tf),
        TargetFilter::And { filters } => filters.iter().find_map(typed_leg),
        _ => None,
    }
}

fn has_type(tf: &TypedFilter, ty: TypeFilter) -> bool {
    tf.type_filters.iter().any(|candidate| candidate == &ty)
}

/// CR 608.2c + CR 122.1: a verbless second conjunct led by a number, not an article,
/// still takes the "put" of the first one (Stand Together).
#[test]
fn counted_counter_conjunct_carries_put_forward() {
    let def = parse_effect_chain(
        "Put two +1/+1 counters on target creature and two +1/+1 counters on another target creature.",
        AbilityKind::Spell,
    );

    let second = assert_plus_counter_node(&def, 2, false).expect("second counter node");
    assert!(
        assert_plus_counter_node(second, 2, true).is_none(),
        "counter chain should contain exactly two nodes",
    );
}

/// CR 608.2c + CR 122.1: each conjunct keeps its own counter kind, count and
/// target (Jean-Luc Picard, Luminary).
#[test]
fn counted_counter_conjunct_keeps_its_own_kind_and_target() {
    let def = parse_effect_chain(
        "Put a +1/+1 counter on up to one other target creature and two charge counters on up to one target Spacecraft.",
        AbilityKind::Spell,
    );

    let second = assert_plus_counter_node(&def, 1, true).expect("second counter node");
    match second.effect.as_ref() {
        Effect::PutCounter {
            counter_type,
            count: QuantityExpr::Fixed { value: 2 },
            target,
        } => {
            assert_eq!(*counter_type, CounterType::Generic("charge".to_string()));
            let tf = typed_leg(target).expect("counter target should be typed");
            assert!(has_type(tf, TypeFilter::Subtype("Spacecraft".to_string())));
        }
        other => panic!("expected two charge counters, got {other:?}"),
    }
    assert!(
        second.multi_target.is_some(),
        "\"up to one\" keeps its own quantifier"
    );
}

/// CR 108.3 + CR 400.3: "the owner of <target phrase>" is the actor and the object
/// is the target: the owner draws, then the targeted creature goes into its owner's
/// library and that library is shuffled (Unification, chapter I).
#[test]
fn owner_of_target_subject_draws_then_shuffles_that_creature() {
    let def = parse_effect_chain(
        "The owner of up to one target creature draws two cards, then shuffles that creature into their library.",
        AbilityKind::Spell,
    );
    assert!(matches!(def.effect.as_ref(), Effect::TargetOnly { .. }));
    assert!(
        def.multi_target.is_some(),
        "\"up to one\" keeps its quantifier"
    );
    let draw = def.sub_ability.as_deref().expect("draw");
    assert!(matches!(
        draw.effect.as_ref(),
        Effect::Draw {
            target: TargetFilter::ParentTargetOwner,
            ..
        }
    ));
    let moved = draw.sub_ability.as_deref().expect("move to library");
    assert!(matches!(
        moved.effect.as_ref(),
        Effect::ChangeZone {
            destination: Zone::Library,
            target: TargetFilter::ParentTarget,
            owner_library: true,
            ..
        }
    ));
}

/// CR 707.10 + CR 115.1: Ulalek, Fused Atrocity — "copy all spells you
/// control, then copy all other activated and triggered abilities you control"
/// targets nothing: one member-driven `CopySpell { ParentTarget }` whose
/// `repeat_for` counts the spells, then the abilities, on the stack.
#[test]
fn effect_copy_all_stack_class_ulalek() {
    let parsed = parse_oracle_text(
        "Devoid (This card has no color.)\n\
         Whenever you cast an Eldrazi spell, you may pay {C}{C}. If you do, copy all spells \
         you control, then copy all other activated and triggered abilities you control. \
         You may choose new targets for the copies. (Mana abilities can't be copied.)",
        "Ulalek, Fused Atrocity",
        &[],
        &["Creature".to_string()],
        &["Eldrazi".to_string()],
    );
    let execute = parsed.triggers[0].execute.as_deref().expect("execute");
    let copy = execute
        .sub_ability
        .as_deref()
        .expect("the copy after the payment");
    assert!(
        matches!(
            &*copy.effect,
            Effect::CopySpell {
                target: TargetFilter::ParentTarget,
                retarget: CopyRetargetPermission::MayChooseNewTargets,
                ..
            }
        ),
        "untargeted copy with new targets allowed, got {:?}",
        copy.effect
    );
    let spells = TargetFilter::And {
        filters: vec![
            TargetFilter::StackSpell,
            TargetFilter::Typed(TypedFilter::default().controller(ControllerRef::You)),
        ],
    };
    let abilities = TargetFilter::StackAbility {
        controller: Some(ControllerRef::You),
        tag: None,
        kind: None,
    };
    assert_eq!(
        copy.repeat_for,
        Some(QuantityExpr::Ref {
            qty: QuantityRef::ObjectCount {
                filter: TargetFilter::Or {
                    filters: vec![spells, abilities],
                },
            },
        }),
        "spells, then abilities, in one loop"
    );
    assert!(copy.sub_ability.is_none(), "nothing after the copies");
}

/// CR 122.6: a counter rider on a search's put step reaches the ChangeZone
/// (Captain Jean-Luc Picard), and a counter phrase in the search filter does not.
#[test]
fn search_put_step_keeps_enter_with_counters() {
    use crate::types::ability::AbilityKind;

    fn put_counters(text: &str) -> Vec<Vec<(CounterType, QuantityExpr)>> {
        let def = parse_effect_chain(text, AbilityKind::Activated);
        let mut found = Vec::new();
        let mut node = Some(&def);
        while let Some(current) = node {
            if let Effect::ChangeZone {
                enter_with_counters,
                ..
            } = &*current.effect
            {
                found.push(enter_with_counters.clone());
            }
            node = current.sub_ability.as_deref();
        }
        found
    }

    assert_eq!(
        put_counters(
            "Search your library for a Spacecraft card, put it onto the battlefield with seven charge counters on it, then shuffle."
        ),
        vec![vec![(
            CounterType::Generic("charge".to_string()),
            QuantityExpr::Fixed { value: 7 }
        )]]
    );
    assert_eq!(
        put_counters(
            "Search your library for a creature card, put it onto the battlefield, then shuffle."
        ),
        vec![vec![]]
    );
}

/// CR 614.1a + CR 611.2a: Brad Boimler's resolved counter replacement floats until
/// end of turn, adds one counter and only covers permanents its controller controls.
#[test]
fn until_end_of_turn_counter_replacement_floats_and_stays_scoped() {
    use crate::types::ability::{AbilityKind, ControllerRef, QuantityModification};

    let def = parse_effect_chain(
        "Until end of turn, if one or more counters would be put on a permanent you control, that many plus one of each of those kinds of counters are put on that permanent instead.",
        AbilityKind::Spell,
    );
    assert_eq!(def.duration, Some(Duration::UntilEndOfTurn));
    let Effect::AddTargetReplacement {
        replacement,
        target: TargetFilter::None,
    } = &*def.effect
    else {
        panic!("expected a floating replacement, got {:?}", def.effect);
    };
    assert_eq!(
        replacement.quantity_modification,
        Some(QuantityModification::Plus { value: 1 })
    );
    assert_eq!(replacement.counter_match, None);
    let Some(TargetFilter::Typed(filter)) = &replacement.valid_card else {
        panic!("expected a typed scope, got {:?}", replacement.valid_card);
    };
    assert_eq!(filter.controller, Some(ControllerRef::You));
}

/// CR 702.26a: both the source and the optional target phase out; the target
/// is optional, the source isn't.
#[test]
fn self_and_up_to_one_target_phase_out() {
    let def = parse_effect_chain(
        "it and up to one target creature you control phase out",
        AbilityKind::Spell,
    );
    assert!(
        matches!(*def.effect, Effect::PhaseOut { .. }),
        "{:?}",
        def.effect
    );
    assert!(matches!(
        def.multi_target.as_ref().map(|m| &m.min),
        Some(QuantityExpr::Fixed { value: 0 })
    ));
    let sub = def.sub_ability.as_deref().expect("source half");
    assert!(matches!(
        *sub.effect,
        Effect::PhaseOut {
            target: TargetFilter::SelfRef
        }
    ));
}

/// CR 115.1 + CR 109.4: kicked Galadriel's Dismissal targets the player, and
/// each creature that player controls phases out; no creature is targeted.
#[test]
fn each_creature_target_player_controls_phases_out_targets_only_the_player() {
    let def = parse_effect_chain(
        "each creature target player controls phases out",
        AbilityKind::Spell,
    );
    assert!(
        matches!(&*def.effect, Effect::PhaseOut { .. }),
        "{:?}",
        def.effect
    );
    assert_eq!(extract_target_filter_from_effect(&def.effect), None);
}

/// CR 603.4 + CR 102.1: Battle Angels of Tyr's three conditions are each a
/// strict lead of the damaged player, checked as the trigger resolves; none is
/// dropped.
#[test]
fn battle_angels_of_tyr_conditions_are_that_players_strict_lead() {
    let parsed = parse_oracle_text(
        "Flying, myriad\nWhenever this creature deals combat damage to a player, draw a card if that player has more cards in hand than each other player. Then you create a Treasure token if that player controls more lands than each other player. Then you gain 3 life if that player has more life than each other player.",
        "Battle Angels of Tyr",
        &["flying".to_string(), "myriad".to_string()],
        &["Creature".to_string()],
        &["Angel".to_string(), "Knight".to_string()],
    );
    let trigger = parsed
        .triggers
        .iter()
        .find(|t| t.mode == crate::types::triggers::TriggerMode::DamageDone)
        .expect("the combat damage trigger");
    let mut node = trigger.execute.as_deref();
    let mut effects = Vec::new();
    while let Some(def) = node {
        let leader = matches!(
            &def.condition,
            Some(AbilityCondition::QuantityCheck {
                lhs: QuantityExpr::Ref {
                    qty: QuantityRef::PlayerCount { .. }
                },
                comparator: Comparator::EQ,
                rhs: QuantityExpr::Fixed { value: 1 },
            })
        );
        assert!(leader, "{:?}", def.condition);
        // CR 608.2c: each later sentence is evaluated on its own guard when the
        // one before it is false.
        if !effects.is_empty() {
            assert_eq!(
                def.sibling_condition,
                crate::types::ability::SiblingCondition::ReplicatedOrBranch
            );
        }
        effects.push(std::mem::discriminant(&*def.effect));
        node = def.sub_ability.as_deref();
    }
    assert_eq!(effects.len(), 3);
}

/// CR 614.1a + CR 500.7: "If an opponent would begin an extra turn" skips only
/// an opponent's extra turn; "If a player would" leaves the scope open.
#[test]
fn extra_turn_skip_is_scoped_to_its_player() {
    let opponent = parse_oracle_text(
        "If an opponent would begin an extra turn, that player skips that turn instead.",
        "Trouble in Pairs",
        &[],
        &["Enchantment".to_string()],
        &[],
    );
    assert_eq!(
        opponent.replacements[0].valid_player,
        Some(crate::types::ability::ReplacementPlayerScope::Opponent)
    );
    let any = parse_oracle_text(
        "If a player would begin an extra turn, that player skips that turn instead.",
        "Ugin's Nexus",
        &[],
        &["Artifact".to_string()],
        &[],
    );
    assert_eq!(any.replacements[0].valid_player, None);
}

/// CR 608.2c + CR 608.2d: "choose any number of <objects> with different
/// <qualities>" is a resolution-time battlefield choice, not targeting: its
/// quantifier is the selection's own cardinality, the relation travels as the
/// set constraint, and "those creatures" after it names the chosen set.
#[test]
fn choose_any_number_with_different_quality_is_a_constrained_object_choice() {
    use crate::types::ability::SharedQualityRelation;
    for (text, quality) in [
        (
            "choose any number of creatures with different powers. Those creatures gain double strike until end of turn.",
            SharedQuality::Power,
        ),
        (
            "choose any number of creatures with different mana values. Destroy those creatures.",
            SharedQuality::ManaValue,
        ),
    ] {
        let def = parse_effect_chain(text, AbilityKind::Spell);
        let Effect::ChooseObjectsIntoTrackedSet {
            chooser,
            filter,
            min,
            max,
            selection_constraint,
            ..
        } = &*def.effect
        else {
            panic!("{text}: expected ChooseObjectsIntoTrackedSet, got {:?}", def.effect);
        };
        assert_eq!(*chooser, TargetFilter::Controller, "{text}");
        assert_eq!(*filter, TargetFilter::Typed(TypedFilter::creature()), "{text}");
        assert_eq!((*min, *max), (0, None), "{text}: \"any number of\" → 0..");
        assert_eq!(
            *selection_constraint,
            SearchSelectionConstraint::SharesQualities {
                qualities: vec![quality],
                relation: SharedQualityRelation::DoesNotShare,
            },
            "{text}"
        );
        assert!(def.multi_target.is_none(), "{text}: the choice declares no targets");
        let sub = def.sub_ability.as_deref().expect("the follow-up instruction");
        let reads_set = match &*sub.effect {
            Effect::GenericEffect {
                static_abilities, ..
            } => static_abilities.iter().all(|s| {
                matches!(s.affected, Some(TargetFilter::TrackedSet { .. }))
            }),
            Effect::Destroy { target, .. } => matches!(target, TargetFilter::TrackedSet { .. }),
            other => panic!("{text}: unexpected follow-up {other:?}"),
        };
        assert!(reads_set, "{text}: \"those creatures\" names the chosen set: {:?}", sub.effect);
    }
    // Without the relation, "choose any number of creatures" keeps its parse.
    let plain = parse_effect_chain(
        "choose any number of creatures with flying",
        AbilityKind::Spell,
    );
    assert!(!matches!(
        &*plain.effect,
        Effect::ChooseObjectsIntoTrackedSet {
            selection_constraint: SearchSelectionConstraint::SharesQualities { .. },
            ..
        }
    ));
}
