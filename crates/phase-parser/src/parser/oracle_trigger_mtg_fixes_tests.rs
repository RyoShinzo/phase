//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `oracle_trigger_tests.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::parser::test_support::assert_no_unimplemented;
use crate::types::ability::filter_contains_filter_prop;
use crate::types::ability::{
    AbilityCondition, AbilityCost, Comparator, ControllerRef, Effect, FilterProp, PtStat,
    PtValueScope, QuantityExpr, QuantityRef, TargetFilter, TriggerCondition, TypedFilter,
};
use crate::types::zones::Zone;

const KOZILEKS_UNSEALING_LIST_LINE: &str = "Whenever you cast a creature spell with mana value 4, 5, or 6, create two 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this token: Add {C}.\"";

const SARKHANS_UNSEALING_LIST_LINE: &str = "Whenever you cast a creature spell with power 4, 5, or 6, this enchantment deals 4 damage to any target.";

const KOZILEKS_UNSEALING_SEVEN_LINE: &str =
    "Whenever you cast a creature spell with mana value 7 or greater, draw three cards.";

/// CR 603.1: the commas inside an exact-value number list in the trigger
/// condition are list separators; the condition/effect boundary is the comma
/// after the list's last leg.
#[test]
fn find_effect_boundary_skips_number_list_commas() {
    for (line, expected) in [
        (SARKHANS_UNSEALING_LIST_LINE, ", this enchantment deals"),
        (KOZILEKS_UNSEALING_LIST_LINE, ", create two"),
        // Bakersbane Duo: a single number right before the effect comma is not
        // a list, so the boundary stays at that comma.
        (
            "Whenever you expend 4, this creature gets +1/+1 until end of turn. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
            ", this creature gets",
        ),
    ] {
        let lower = line.to_lowercase();
        let boundary = find_effect_boundary(&lower).expect("effect boundary");
        let suffix = &lower[boundary..];
        assert_eq!(
            &suffix[..expected.len()],
            expected,
            "boundary of {line:?} should precede {expected:?}, got {suffix:?}"
        );
    }
}

const SARKHANS_UNSEALING_SEVEN_LINE: &str = "Whenever you cast a creature spell with power 7 or greater, this enchantment deals 4 damage to each opponent and each creature and planeswalker they control.";

/// CR 208.1 + CR 603.1: Sarkhan's Unsealing's first trigger reads "power 4,
/// 5, or 6" whole — an `AnyOf` of three power EQ comparisons — and its effect
/// is the plain 4 damage to any target, with no `Unimplemented` link.
#[test]
fn sarkhans_unsealing_power_value_list_trigger() {
    let trigger = parse_trigger_line(SARKHANS_UNSEALING_LIST_LINE, "Sarkhan's Unsealing");
    assert_eq!(trigger.mode, TriggerMode::SpellCast);
    let power_eq = |value| FilterProp::PtComparison {
        stat: PtStat::Power,
        scope: PtValueScope::Current,
        comparator: Comparator::EQ,
        value: QuantityExpr::Fixed { value },
    };
    let expected = FilterProp::AnyOf {
        props: vec![power_eq(4), power_eq(5), power_eq(6)],
    };
    let valid_card = trigger.valid_card.as_ref().expect("spell filter");
    assert!(
        filter_contains_filter_prop(valid_card, &|prop| *prop == expected),
        "valid_card must hold the power value list, got {valid_card:?}"
    );
    let execute = trigger.execute.as_deref().expect("trigger body");
    assert_no_unimplemented(execute);
    assert!(
        matches!(
            execute.effect.as_ref(),
            Effect::DealDamage {
                amount: QuantityExpr::Fixed { value: 4 },
                target: TargetFilter::Any,
                ..
            }
        ),
        "expected 4 damage to any target, got {:?}",
        execute.effect
    );

    // Reach-guard: the 7+ line still parses through the same path.
    let seven = parse_trigger_line(SARKHANS_UNSEALING_SEVEN_LINE, "Sarkhan's Unsealing");
    assert_eq!(seven.mode, TriggerMode::SpellCast);
    let seven_card = seven.valid_card.as_ref().expect("spell filter");
    assert!(
        filter_contains_filter_prop(seven_card, &|prop| *prop
            == FilterProp::PtComparison {
                stat: PtStat::Power,
                scope: PtValueScope::Current,
                comparator: Comparator::GE,
                value: QuantityExpr::Fixed { value: 7 },
            }),
        "7+ line must keep power GE 7, got {seven_card:?}"
    );
}

/// CR 202.3 + CR 603.1: Kozilek's Unsealing's first trigger reads "mana value
/// 4, 5, or 6" whole — an `AnyOf` of three mana-value EQ comparisons — and its
/// effect creates two Eldrazi Spawn, with no `Unimplemented` link.
#[test]
fn kozileks_unsealing_mana_value_list_trigger() {
    let trigger = parse_trigger_line(KOZILEKS_UNSEALING_LIST_LINE, "Kozilek's Unsealing");
    assert_eq!(trigger.mode, TriggerMode::SpellCast);
    let mv_eq = |value| FilterProp::Cmc {
        comparator: Comparator::EQ,
        value: QuantityExpr::Fixed { value },
    };
    let expected = FilterProp::AnyOf {
        props: vec![mv_eq(4), mv_eq(5), mv_eq(6)],
    };
    let valid_card = trigger.valid_card.as_ref().expect("spell filter");
    assert!(
        filter_contains_filter_prop(valid_card, &|prop| *prop == expected),
        "valid_card must hold the mana value list, got {valid_card:?}"
    );
    let execute = trigger.execute.as_deref().expect("trigger body");
    assert_no_unimplemented(execute);
    let Effect::Token { name, count, .. } = execute.effect.as_ref() else {
        panic!("expected a token effect, got {:?}", execute.effect);
    };
    assert_eq!(name, "Eldrazi Spawn");
    assert_eq!(*count, QuantityExpr::Fixed { value: 2 });

    // Reach-guard: the 7+ line still parses through the same path.
    let seven = parse_trigger_line(KOZILEKS_UNSEALING_SEVEN_LINE, "Kozilek's Unsealing");
    assert_eq!(seven.mode, TriggerMode::SpellCast);
    let seven_card = seven.valid_card.as_ref().expect("spell filter");
    assert!(
        filter_contains_filter_prop(seven_card, &|prop| *prop
            == FilterProp::Cmc {
                comparator: Comparator::GE,
                value: QuantityExpr::Fixed { value: 7 },
            }),
        "7+ line must keep mana value GE 7, got {seven_card:?}"
    );
}

/// CR 701.6a + CR 118.12a: on a counter, "unless they pay" asks the countered
/// spell's controller, not the trigger's event player, who on a cast trigger is
/// the source's own caster.
#[test]
fn trigger_counter_unless_they_pay_binds_countered_spell_controller() {
    let def = parse_trigger_line(
        "When you cast this spell, counter target spell an opponent controls unless they pay {1}.",
        "Hope-Ender Coatl",
    );

    let unless_pay = def.unless_pay.as_ref().expect("should have unless_pay");
    assert_eq!(unless_pay.payer, TargetFilter::ParentTargetController);
    assert!(matches!(unless_pay.cost, AbilityCost::Mana { .. }));
}

/// CR 603.4 + CR 608.2k: an intervening-`if` that tests the source's own
/// characteristics names the source, so the effect's bare "it" is the source,
/// not the spell whose cast triggered the ability. Romulan Bird-of-Prey: "put
/// two charge counters on it" goes on the Bird, negated condition or not.
#[test]
fn spell_cast_intervening_if_on_source_characteristics_binds_it_to_source() {
    for line in [
        "Whenever you cast a noncreature spell, if this Spacecraft isn't a creature, put two charge counters on it.",
        "Whenever you cast a noncreature spell, if this Spacecraft is a creature, put two charge counters on it.",
    ] {
        let def = parse_trigger_line(line, "Romulan Bird-of-Prey");
        assert_eq!(def.mode, TriggerMode::SpellCast, "{line}");
        let execute = def.execute.as_ref().expect("trigger must execute");
        assert!(
            matches!(
                &*execute.effect,
                Effect::PutCounter { target: TargetFilter::SelfRef, .. }
            ),
            "\"on it\" after an intervening-if on the source must bind SelfRef: {line} -> {:?}",
            execute.effect
        );
    }
}

/// CR 106.1b + CR 601.2h: "{C}" names colorless mana, a type but no color, so
/// the clause reads the payment record's colorless amount, at least one per
/// `{C}` symbol, against the object the anaphor names.
#[test]
fn extract_colorless_mana_spent_condition() {
    let colorless = |scope, value| TriggerCondition::QuantityComparison {
        lhs: QuantityExpr::Ref {
            qty: QuantityRef::ManaSpentToCast {
                scope,
                metric: crate::types::ability::CastManaSpentMetric::Colorless,
            },
        },
        comparator: Comparator::GE,
        rhs: QuantityExpr::Fixed { value },
    };
    let (cleaned, cond) = extract_if_condition(
        "if {C} was spent to cast it, create two 0/1 colorless Eldrazi Spawn creature tokens",
    );
    assert_eq!(
        cleaned,
        "create two 0/1 colorless Eldrazi Spawn creature tokens"
    );
    assert_eq!(
        cond,
        Some(colorless(CastManaObjectScope::TriggeringSpell, 1))
    );

    let (cleaned, cond) = extract_if_condition("draw a card if {C}{C} was spent to cast ~");
    assert_eq!(cleaned, "draw a card");
    assert_eq!(cond, Some(colorless(CastManaObjectScope::SelfObject, 2)));
}

/// CR 603.4: Drowner of Truth's cast trigger hoists "if {C} was spent to cast
/// it" as its intervening-if and keeps the token creation as its effect.
#[test]
fn trigger_colorless_mana_spent_intervening_if() {
    let def = parse_trigger_line(
        "When you cast this spell, if {C} was spent to cast it, create two 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this token: Add {C}.\"",
        "Drowner of Truth",
    );
    assert_eq!(def.mode, TriggerMode::SpellCast);
    assert!(
        matches!(
            def.condition,
            Some(TriggerCondition::QuantityComparison {
                lhs: QuantityExpr::Ref {
                    qty: QuantityRef::ManaSpentToCast {
                        scope: CastManaObjectScope::TriggeringSpell,
                        metric: crate::types::ability::CastManaSpentMetric::Colorless,
                    },
                },
                comparator: Comparator::GE,
                rhs: QuantityExpr::Fixed { value: 1 },
            })
        ),
        "got {:?}",
        def.condition
    );
    match &*def.execute.as_ref().unwrap().effect {
        Effect::Token { .. } => {}
        other => panic!("expected Token effect, got {other:?}"),
    }
}

/// CR 303.4b + CR 603.2: Tenuous Truce. "Enchanted opponent" names the enchanted
/// player as "enchanted player" does: the end step is only theirs, and "when you
/// attack enchanted opponent or a planeswalker they control or when they attack you
/// or a planeswalker you control" is two attack triggers, one for each direction,
/// each scoped to the enchanted player and taking planeswalkers along.
#[test]
fn tenuous_truce_scopes_end_step_and_both_attack_directions_to_enchanted_opponent() {
    use crate::types::triggers::AttackTargetFilter;
    let defs = parse_trigger_lines(
        "At the beginning of enchanted opponent's end step, you and that player each draw a card.",
        "Tenuous Truce",
    );
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].mode, TriggerMode::Phase);
    assert_eq!(defs[0].phase, Some(crate::types::phase::Phase::End));
    assert_eq!(
        defs[0].valid_target,
        Some(TargetFilter::AttachedTo),
        "only the enchanted opponent's end step"
    );

    let defs = parse_trigger_lines(
        "When you attack enchanted opponent or a planeswalker they control or when they attack you or a planeswalker you control, sacrifice this Aura.",
        "Tenuous Truce",
    );
    assert_eq!(defs.len(), 2, "{defs:#?}");
    let ours = &defs[0];
    assert_eq!(ours.mode, TriggerMode::Attacks);
    assert_eq!(ours.valid_source, Some(TargetFilter::Controller));
    assert_eq!(ours.valid_target, Some(TargetFilter::AttachedTo));
    assert_eq!(
        ours.attack_target_filter,
        Some(AttackTargetFilter::PlayerOrPlaneswalker)
    );
    let theirs = &defs[1];
    assert_eq!(theirs.mode, TriggerMode::Attacks);
    assert_eq!(
        theirs.valid_source,
        Some(TargetFilter::Typed(
            TypedFilter::default().controller(ControllerRef::EnchantedPlayer)
        )),
        "the attacking player is the enchanted opponent"
    );
    assert_eq!(theirs.valid_target, Some(TargetFilter::Controller));
    assert_eq!(
        theirs.attack_target_filter,
        Some(AttackTargetFilter::PlayerOrPlaneswalker)
    );
    for def in &defs {
        assert!(
            matches!(
                def.execute.as_ref().map(|e| e.effect.as_ref()),
                Some(Effect::Sacrifice {
                    target: TargetFilter::SelfRef,
                    ..
                })
            ),
            "{:?}",
            def.execute
        );
    }
}

/// CR 303.4b: "you attack enchanted opponent" is the defender-scoped attack
/// trigger "you attack enchanted player" is (Archnemesis), not a bare `YouAttack`.
#[test]
fn you_attack_enchanted_opponent_is_defender_scoped() {
    use crate::types::triggers::AttackTargetFilter;
    let def = parse_trigger_line(
        "Whenever you attack enchanted opponent, draw a card.",
        "Probe",
    );
    assert_eq!(def.mode, TriggerMode::Attacks);
    assert_eq!(def.valid_source, Some(TargetFilter::Controller));
    assert_eq!(def.valid_target, Some(TargetFilter::AttachedTo));
    assert_eq!(def.attack_target_filter, Some(AttackTargetFilter::Player));
}

/// CR 303.4b: Overencumbered — "At the beginning of combat on enchanted
/// opponent's turn" is only the enchanted opponent's combat.
#[test]
fn overencumbered_beginning_of_combat_only_on_enchanted_opponents_turn() {
    let defs = parse_trigger_lines(
        "At the beginning of combat on enchanted opponent's turn, that player may pay {1} for each artifact they control. If they don't, creatures can't attack this combat.",
        "Overencumbered",
    );
    assert_eq!(defs[0].mode, TriggerMode::Phase);
    assert_eq!(defs[0].phase, Some(crate::types::phase::Phase::BeginCombat));
    assert_eq!(defs[0].valid_target, Some(TargetFilter::AttachedTo));
}

/// CR 608.2c + CR 701.25a: Enlightened Confidant — the surveil's reflexive gate
/// keeps its dynamic mana-value bound ("with mana value less than or equal to
/// the amount of life you gained this turn") although "into your graveyard this
/// way" follows it, and "that card" is the surveilled card, not the Confidant.
#[test]
fn enlightened_confidant_surveil_gate_keeps_life_gained_bound() {
    let def = parse_trigger_line(
        "At the beginning of your end step, if you gained life this turn, surveil 1. If you put a card with mana value less than or equal to the amount of life you gained this turn into your graveyard this way, put that card into your hand.",
        "Enlightened Confidant",
    );
    let execute = def.execute.expect("execute");
    assert!(
        matches!(*execute.effect, Effect::Surveil { .. }),
        "{:?}",
        execute.effect
    );
    let sub = execute.sub_ability.expect("the gated return");
    let Some(AbilityCondition::ZoneChangedThisWay {
        filter,
        destination,
        ..
    }) = &sub.condition
    else {
        panic!(
            "expected a ZoneChangedThisWay gate, got {:?}",
            sub.condition
        );
    };
    assert_eq!(*destination, Some(Zone::Graveyard));
    let TargetFilter::Typed(typed) = filter else {
        panic!("expected a typed filter, got {filter:?}");
    };
    assert!(
        typed.properties.iter().any(|prop| matches!(
            prop,
            FilterProp::Cmc {
                comparator: Comparator::LE,
                value: QuantityExpr::Ref {
                    qty: QuantityRef::LifeGainedThisTurn { .. }
                },
            }
        )),
        "{:?}",
        typed.properties
    );
    assert!(
        matches!(
            &*sub.effect,
            Effect::ChangeZone {
                target: TargetFilter::LastZoneChanged,
                destination: Zone::Hand,
                ..
            }
        ),
        "{:?}",
        sub.effect
    );
}

/// CR 608.2c + CR 701.21a: Sephiroth, One-Winged Angel's "draw that many cards" is the
/// number of creatures sacrificed, not the attack trigger's own event amount.
#[test]
fn sephiroth_attack_draws_the_number_sacrificed() {
    let def = parse_trigger_line(
        "Whenever Sephiroth attacks, you may sacrifice any number of other creatures. If you do, draw that many cards.",
        "Sephiroth, One-Winged Angel",
    );
    let execute = def.execute.expect("execute");
    assert!(
        matches!(
            &*execute.effect,
            Effect::Sacrifice {
                count: QuantityExpr::UpTo { .. },
                ..
            }
        ),
        "{:?}",
        execute.effect
    );
    let sub = execute.sub_ability.expect("the draw");
    let Effect::Draw { count, .. } = &*sub.effect else {
        panic!("expected a draw, got {:?}", sub.effect);
    };
    assert_eq!(
        *count,
        QuantityExpr::Ref {
            qty: QuantityRef::PreviousEffectCount
        }
    );
}
