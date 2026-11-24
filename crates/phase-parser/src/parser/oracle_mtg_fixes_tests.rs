//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `oracle_tests.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::test_helpers::parse;
use super::*;
use crate::parser::oracle_effect::parse_effect_chain;
use crate::types::ability::{
    ContinuousModification, ControllerRef, Effect, FilterProp,
    ManaSpendRestriction, QuantityExpr, QuantityRef, StaticCondition, TargetFilter,
};
use crate::types::counter::{CounterMatch, CounterType};
use crate::types::keywords::{FlashbackCost, Keyword, KeywordKind};
use crate::types::mana::ManaCost;
use crate::types::statics::StaticMode;
use crate::types::triggers::TriggerMode;
use crate::types::zones::Zone;

/// Parse with raw MTGJSON keyword names (for testing keyword extraction).
fn parse_with_keyword_names(
    text: &str,
    name: &str,
    keyword_names: &[&str],
    types: &[&str],
    subtypes: &[&str],
) -> ParsedAbilities {
    let keyword_names: Vec<String> = keyword_names.iter().map(|s| s.to_string()).collect();
    let types: Vec<String> = types.iter().map(|s| s.to_string()).collect();
    let subtypes: Vec<String> = subtypes.iter().map(|s| s.to_string()).collect();
    parse_oracle_text(text, name, &keyword_names, &types, &subtypes)
}

#[test]
fn mana_spend_restriction_a_spell_with_devoid() {
    // CR 106.6 + CR 702.114a: Corrupted Crossroads.
    use crate::parser::oracle_effect::mana::parse_mana_spend_restriction;
    let result = parse_mana_spend_restriction("spend this mana only to cast a spell with devoid");
    assert_eq!(
        result.and_then(|(mut restrictions, _)| (restrictions.len() == 1)
            .then(|| restrictions.pop().expect("one restriction"))),
        Some(ManaSpendRestriction::SpellWithKeywordKind(
            KeywordKind::Devoid
        ))
    );
}

/// CR 608.2c + CR 725.1: "that player becomes the monarch" in a damage trigger
/// crowns the damaged player (Starscream, Seeker Leader), not the ability's
/// controller. The subject is a player anaphor with no target slot, which the
/// `target` axis used to drop to the controller default.
#[test]
fn that_player_becomes_the_monarch_names_the_damaged_player_cr_608_2c() {
    let parsed = parse(
        "Whenever Starscream deals combat damage to a player, if there is no monarch, that player becomes the monarch.",
        "Starscream, Seeker Leader",
        &[],
        &["Artifact"],
        &["Vehicle"],
    );
    let trigger = parsed
        .triggers
        .first()
        .expect("the damage trigger must parse");
    let execute = trigger
        .execute
        .as_ref()
        .expect("the trigger must have an execute body");
    assert_eq!(
        *execute.effect,
        Effect::BecomeMonarch {
            target: TargetFilter::TriggeringPlayer
        },
        "the damaged player must become the monarch"
    );
}

/// CR 700.2: "Whenever you face a dilemma" is a real trigger, and both forms of
/// the per-turn count read the same counter (Star Trek release notes).
#[test]
fn face_a_dilemma_parses() {
    let parsed = parse_oracle_text(
        "Whenever you face a dilemma, draw a card.",
        "Dilemma Test",
        &[],
        &["Creature".to_string()],
        &[],
    );
    assert_eq!(parsed.triggers[0].mode, TriggerMode::DilemmaFaced);
    assert!(parsed.triggers[0].valid_target.is_some());

    let parsed = parse_oracle_text(
        "At the beginning of your end step, create a 1/1 red Officer creature token for each dilemma you've faced this turn.",
        "Dilemma Test",
        &[],
        &["Creature".to_string()],
        &[],
    );
    assert!(serde_json::to_string(&parsed.triggers[0])
        .unwrap()
        .contains("DilemmasFacedThisTurn"));

    let parsed = parse_oracle_text(
        "At the beginning of each end step, if you faced a dilemma this turn, draw a card.",
        "Dilemma Test",
        &[],
        &["Creature".to_string()],
        &[],
    );
    assert!(serde_json::to_string(&parsed.triggers[0])
        .unwrap()
        .contains("DilemmasFacedThisTurn"));
}

/// Beam me up (`trk`): "Enchanted permanent doesn't untap during its controller's untap
/// step and can't be beamed up." is two statics on the host (Marooned).
#[test]
fn marooned_cant_untap_and_cant_be_beamed_up() {
    let result = parse(
        "Enchant artifact or creature\nWhen this Aura enters, you may pay {3}. If you do, tap enchanted permanent.\nEnchanted permanent doesn't untap during its controller's untap step and can't be beamed up.",
        "Marooned",
        &[],
        &["Enchantment"],
        &["Aura"],
    );
    let modes: Vec<_> = result.statics.iter().map(|def| def.mode.clone()).collect();
    assert_eq!(
        modes,
        vec![
            StaticMode::CantUntap,
            StaticMode::Other("CantBeBeamedUp".to_string())
        ]
    );
}

/// Beam me up (`trk`): the return leg of a printed beam me up cost excludes a permanent
/// that can't be beamed up.
#[test]
fn beam_me_up_return_excludes_cant_be_beamed_up() {
    let result = parse(
        "Draw a card.\nBeam me up {3}{W} (You may cast this card from your graveyard for {3}{W} if you also return a creature you control to its owner's hand. Then exile this spell.)",
        "Probe",
        &[],
        &["Sorcery"],
        &[],
    );
    let Some(Keyword::Flashback(FlashbackCost::NonMana(AbilityCost::Composite { costs }))) =
        result.extracted_keywords.first()
    else {
        panic!(
            "beam me up must be a compound flashback: {:?}",
            result.extracted_keywords
        );
    };
    let Some(AbilityCost::ReturnToHand {
        filter: Some(TargetFilter::Typed(filter)),
        ..
    }) = costs.get(1)
    else {
        panic!("missing return leg: {costs:?}");
    };
    assert!(filter.properties.contains(&FilterProp::Not {
        prop: Box::new(FilterProp::CantBeBeamedUp),
    }));
}

/// Beam me up (`trk`): Scotty, Miracle Worker gives instant and sorcery cards in the
/// graveyard beam me up during your main phase, for their mana cost, and the return leg
/// excludes Scotty as the granter (CR 201.5a).
#[test]
fn scotty_grants_beam_me_up_for_the_mana_cost() {
    let result = parse(
        "During your main phase, each card in your graveyard has beam me up. The beam me up cost is equal to that spell's mana cost. ~ can't be beamed up to cast a spell this way. (Only instant and sorcery spells can be cast this way. You may cast a card from your graveyard for its beam me up cost if you also return another creature you control to its owner's hand. Then exile that spell.)",
        "Scotty, Miracle Worker",
        &[],
        &["Creature"],
        &["Human", "Artificer"],
    );
    assert!(result.abilities.is_empty(), "{:?}", result.abilities);
    let [grant] = result.statics.as_slice() else {
        panic!("expected one static: {:?}", result.statics);
    };
    assert_eq!(
        grant.condition,
        Some(StaticCondition::And {
            conditions: vec![
                StaticCondition::DuringYourTurn,
                StaticCondition::CurrentPhaseIs {
                    phases: vec![Phase::PreCombatMain, Phase::PostCombatMain],
                },
            ],
        })
    );
    let [ContinuousModification::AddKeyword {
        keyword: Keyword::Flashback(FlashbackCost::NonMana(AbilityCost::Composite { costs })),
    }] = grant.modifications.as_slice()
    else {
        panic!("expected a beam me up grant: {:?}", grant.modifications);
    };
    assert_eq!(
        costs[0],
        AbilityCost::Mana {
            cost: ManaCost::SelfManaCost
        }
    );
    let AbilityCost::ReturnToHand {
        filter: Some(TargetFilter::And { filters }),
        ..
    } = &costs[1]
    else {
        panic!("missing return leg: {costs:?}");
    };
    assert!(filters.contains(&TargetFilter::Not {
        filter: Box::new(TargetFilter::GrantingObject { bound: None }),
    }));
}

/// Assimilate (`trk`) + CR 607.2d: chapter I of The Best of Both Worlds remembers the
/// creature it assimilated, and chapter III exiles exactly that creature.
#[test]
fn best_of_both_worlds_links_the_assimilated_creature() {
    let result = parse(
        "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — Assimilate target nonartifact creature an opponent controls. (Gain control of it and put a +1/+1 counter on it. It's a Borg artifact creature and loses all other creature types.)\nII — Artifact creatures you control get +2/+2 until end of turn.\nIII — Exile the Borg assimilated with this Saga, then return it to the battlefield under its owner's control.",
        "The Best of Both Worlds",
        &[],
        &["Enchantment"],
        &["Saga"],
    );
    let chapter = |n: usize| {
        let execute = result.triggers[n]
            .execute
            .as_deref()
            .expect("chapter ability");
        std::iter::successors(Some(execute), |def| def.sub_ability.as_deref())
            .map(|def| (*def.effect).clone())
            .collect::<Vec<_>>()
    };
    assert!(matches!(
        chapter(0).first(),
        Some(Effect::GainControl { .. })
    ));
    assert_eq!(
        chapter(0).last(),
        Some(&Effect::RememberCard {
            target: TargetFilter::ParentTarget
        })
    );
    assert!(matches!(
        chapter(2).first(),
        Some(Effect::ChangeZone {
            target: TargetFilter::ChosenCard,
            destination: Zone::Exile,
            ..
        })
    ));
}

/// CR 603.2d: Lwaxana Troi, Charming Guest grants the chosen creature a doubler of
/// its own triggered abilities for as long as she remains tapped.
#[test]
fn lwaxana_troi_grants_the_chosen_creature_trigger_doubling() {
    use crate::types::statics::{StaticMode, TriggerCause};
    let result = parse(
        "You may choose not to untap Lwaxana Troi during your untap step.\n{2}, {T}: Choose another target creature you control. For as long as Lwaxana Troi remains tapped, if a triggered ability of that creature triggers, that ability triggers an additional time.",
        "Lwaxana Troi, Charming Guest",
        &[],
        &["Creature"],
        &["Betazoid", "Advisor"],
    );
    let grant = result.abilities[0]
        .sub_ability
        .as_deref()
        .expect("the grant follows the target");
    let Effect::GenericEffect {
        static_abilities,
        target: Some(TargetFilter::ParentTarget),
        ..
    } = &*grant.effect
    else {
        panic!(
            "expected a grant to the chosen creature, got {:?}",
            grant.effect
        );
    };
    assert!(static_abilities[0].modifications.iter().any(|m| matches!(
        m,
        ContinuousModification::AddStaticMode {
            mode: StaticMode::DoubleTriggers {
                cause: TriggerCause::Any
            }
        }
    )));
}

/// CR 122.1 + CR 608.2c: Holy Frazzle-Cannon puts a +1/+1 counter on the attacking
/// equipped creature and on each other creature you control that shares a creature
/// type with it: a `PutCounter` on the triggering creature, then a `PutCounterAll`
/// over the sharers that leaves the triggering creature out.
#[test]
fn holy_frazzle_cannon_counters_each_creature_sharing_a_type() {
    use crate::types::ability::SharedQuality;
    let result = parse(
        "Whenever equipped creature attacks, put a +1/+1 counter on that creature and each other creature you control that shares a creature type with it.\nEquip {1}",
        "Holy Frazzle-Cannon",
        &[],
        &["Artifact"],
        &["Equipment"],
    );
    let execute = result.triggers[0].execute.as_deref().expect("execute");
    assert!(matches!(
        &*execute.effect,
        Effect::PutCounter {
            target: TargetFilter::TriggeringSource,
            ..
        }
    ));
    let fanout = execute.sub_ability.as_deref().expect("the fan-out follows");
    let Effect::PutCounterAll {
        counter_type: CounterType::Plus1Plus1,
        target: TargetFilter::Typed(typed),
        ..
    } = &*fanout.effect
    else {
        panic!("expected a PutCounterAll fan-out, got {:?}", fanout.effect);
    };
    assert_eq!(typed.controller, Some(ControllerRef::You));
    assert!(typed.properties.contains(&FilterProp::DistinctFrom {
        reference: Box::new(TargetFilter::TriggeringSource)
    }));
    assert!(typed.properties.iter().any(|p| matches!(
        p,
        FilterProp::SharesQuality {
            quality: SharedQuality::CreatureType,
            reference: Some(reference),
            ..
        } if matches!(reference.as_ref(), TargetFilter::TriggeringSource)
    )));
}

/// CR 721.1 + CR 721.2a: a station card that is no Spacecraft — Adagia,
/// Windswept Bastion, a Planet land — gates its "12+ |" ability on twelve
/// charge counters like a Spacecraft does; it parsed with no threshold.
#[test]
fn planet_station_threshold_ability_is_gated_on_charge_counters() {
    let r = parse_with_keyword_names(
        "This land enters tapped.\n{T}: Add {W}.\nStation (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)\n12+ | {3}{W}, {T}: Create a token that's a copy of target artifact or enchantment you control, except it's legendary. Activate only as a sorcery.",
        "Adagia, Windswept Bastion",
        &["station"],
        &["Land"],
        &["Planet"],
    );
    let copy = r
        .abilities
        .iter()
        .find(|a| matches!(*a.effect, Effect::CopyTokenOf { .. }))
        .expect("the 12+ copy ability");
    assert!(
        copy.activation_restrictions.iter().any(|r| matches!(
            r,
            ActivationRestriction::CounterThreshold {
                counters: CounterMatch::OfType(CounterType::Generic(name)),
                minimum: 12,
                maximum: None,
            } if name == "charge"
        )),
        "12+ gate missing: {:?}",
        copy.activation_restrictions
    );
    assert!(copy
        .activation_restrictions
        .iter()
        .any(|r| matches!(r, ActivationRestriction::AsSorcery)));
}

/// CR 111.4 + CR 205.3i: Overlord of the Hauntwoods' token is named Everywhere
/// and has the five basic land types (and so their mana abilities, CR 305.6).
#[test]
fn token_named_everywhere_is_every_basic_land_type() {
    let d = parse_effect_chain(
        "Create a tapped colorless land token named Everywhere that is every basic land type.",
        AbilityKind::Spell,
    );
    let Effect::Token {
        name,
        types,
        tapped,
        colors,
        ..
    } = &*d.effect
    else {
        panic!("expected a token, got {:?}", d.effect);
    };
    assert_eq!(name, "Everywhere");
    assert!(*tapped);
    assert!(colors.is_empty());
    for t in ["Land", "Plains", "Island", "Swamp", "Mountain", "Forest"] {
        assert!(types.iter().any(|x| x == t), "{t} missing from {types:?}");
    }
}

/// CR 608.2c: "Then for each blight counter on it, each opponent loses 4 life
/// unless …" (Rottenmouth Viper) repeats the instruction once per counter; the
/// leading "Then" must not split the repeat count off its instruction.
#[test]
fn then_for_each_counter_repeats_the_instruction() {
    let d = parse_effect_chain(
        "Put a blight counter on it. Then for each blight counter on it, each opponent loses 4 life unless that player sacrifices a nonland permanent of their choice or discards a card.",
        AbilityKind::Spell,
    );
    let sub = d.sub_ability.as_ref().expect("the loss instruction");
    assert!(
        matches!(&*sub.effect, Effect::LoseLife { .. }),
        "{:?}",
        sub.effect
    );
    assert!(sub.unless_pay.is_some());
    assert!(
        matches!(
            &sub.repeat_for,
            Some(QuantityExpr::Ref {
                qty: QuantityRef::CountersOn { .. }
            })
        ),
        "{:?}",
        sub.repeat_for
    );
}
