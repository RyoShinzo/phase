//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `mana.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them.

use super::*;

fn restriction_and_its_source_type(spell_type: &str) -> Vec<ManaSpendRestriction> {
    vec![
        ManaSpendRestriction::ActivateOnly,
        ManaSpendRestriction::SpellTypeOrAbilityActivation {
            spell_type: spell_type.to_string(),
            ability: AbilityActivationScope::OfSpellType,
        },
    ]
}

#[test]
fn activate_abilities_of_a_source_type_keeps_the_type() {
    // CR 106.6: "Spend this mana only to activate abilities of artifact sources"
    // (Steelswarm Operator) is mana for activations of artifacts only, in every
    // spelling of the source phrase.
    for (text, spell_type) in [
        (
            "spend this mana only to activate abilities of artifact sources.",
            "Artifact",
        ),
        (
            "spend this mana only to activate abilities of land sources.",
            "Land",
        ),
        (
            "spend this mana only to activate abilities of creature sources.",
            "Creature",
        ),
        (
            "spend this mana only to activate abilities of artifacts.",
            "Artifact",
        ),
    ] {
        let (restrictions, grants) = parse_mana_spend_restriction(text).expect(text);
        assert_eq!(restrictions, restriction_and_its_source_type(spell_type), "{text}");
        assert!(grants.is_empty(), "{text}");
    }
}

#[test]
fn activate_abilities_with_no_source_stays_activation_only() {
    let (restrictions, _) =
        parse_mana_spend_restriction("spend this mana only to activate abilities.").unwrap();
    assert_eq!(restrictions, vec![ManaSpendRestriction::ActivateOnly]);
}

#[test]
fn source_typed_activation_mana_pays_only_that_source_s_abilities() {
    // CR 106.6: the two restrictions are AND gates on the produced mana: an
    // artifact's ability may be paid, a non-artifact's ability and any spell may not.
    let lowered: Vec<ManaRestriction> = vec![
        ManaRestriction::OnlyForActivation,
        ManaRestriction::OnlyForTypeSpellsOrAbilities {
            spell_type: "Artifact".to_string(),
            ability: AbilityActivationScope::OfSpellType,
        },
    ];
    let artifact = ["Artifact".to_string(), "Creature".to_string()];
    let creature = ["Creature".to_string()];
    let human = ["Human".to_string()];
    assert!(lowered
        .iter()
        .all(|r| r.allows_activation(&artifact, &[], None)));
    assert!(!lowered
        .iter()
        .all(|r| r.allows_activation(&creature, &human, None)));
}
