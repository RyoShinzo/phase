//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `mod tests`.
//! They live in a file of our own so that a merge of an upstream release never meets them.

use super::*;
use crate::types::ability::{ControllerRef, TargetFilter, TypeFilter, TypedFilter};

fn typed(type_filters: Vec<TypeFilter>) -> TargetFilter {
    TargetFilter::Typed(TypedFilter {
        type_filters,
        controller: Some(ControllerRef::You),
        properties: vec![],
    })
}

/// CR 205.2a + CR 205.3: "that's an artifact or Human" (Paladin Danse, Steel Maverick) mixes a
/// card type with a subtype; each leg is its own branch, and neither is dropped.
#[test]
fn relative_clause_card_type_or_subtype() {
    let (filter, rest) = parse_type_phrase_folding("creature you control that's an artifact or Human");
    assert_eq!(rest.trim(), "");
    assert_eq!(
        filter,
        TargetFilter::Or {
            filters: vec![
                typed(vec![TypeFilter::Creature, TypeFilter::Artifact]),
                typed(vec![TypeFilter::Creature, TypeFilter::Subtype("Human".to_string())]),
            ]
        }
    );
}

/// The same clause with the subtype first ("that's a Human or an artifact").
#[test]
fn relative_clause_subtype_or_card_type() {
    let (filter, rest) = parse_type_phrase_folding("creature you control that's a Human or an artifact");
    assert_eq!(rest.trim(), "");
    assert_eq!(
        filter,
        TargetFilter::Or {
            filters: vec![
                typed(vec![TypeFilter::Creature, TypeFilter::Subtype("Human".to_string())]),
                typed(vec![TypeFilter::Creature, TypeFilter::Artifact]),
            ]
        }
    );
}

/// Pure lists keep their own parsers: subtypes AND-merge as one `AnyOf`, card types branch.
#[test]
fn relative_clause_pure_lists_unchanged() {
    let (filter, _) = parse_type_phrase_folding("creature you control that's an Ape or a Monkey");
    assert_eq!(
        filter,
        typed(vec![
            TypeFilter::AnyOf(vec![
                TypeFilter::Subtype("Ape".to_string()),
                TypeFilter::Subtype("Monkey".to_string()),
            ]),
            TypeFilter::Creature,
        ])
    );
    let (filter, _) = parse_type_phrase_folding("permanent you control that's an artifact or creature");
    assert_eq!(
        filter,
        TargetFilter::Or {
            filters: vec![
                typed(vec![TypeFilter::Permanent, TypeFilter::Artifact]),
                typed(vec![TypeFilter::Permanent, TypeFilter::Creature]),
            ]
        }
    );
}

/// The whole of Paladin Danse's ability: the granted indestructible reaches both legs.
#[test]
fn paladin_danse_grant_reaches_artifacts_and_humans() {
    let parsed = crate::parser::oracle::parse_oracle_text(
        "Vigilance, lifelink\nExile Paladin Danse: Each creature you control that's an artifact or Human gains indestructible until end of turn.",
        "Paladin Danse, Steel Maverick",
        &["Vigilance".to_string(), "Lifelink".to_string()],
        &["Artifact".to_string(), "Creature".to_string()],
        &["Synth".to_string(), "Knight".to_string()],
    );
    let json = serde_json::to_string(&parsed.abilities).expect("serialise");
    assert!(json.contains("\"Human\""), "Human leg dropped: {json}");
    assert!(json.contains("\"Artifact\""), "Artifact leg dropped: {json}");
}
