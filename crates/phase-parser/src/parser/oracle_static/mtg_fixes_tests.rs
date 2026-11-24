//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `snapshot_tests.rs`, `tests.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::prelude::*;
use super::*;
use crate::types::ability::{ControllerRef, FilterProp, TargetFilter, TypedFilter};
use crate::types::keywords::Keyword;
use crate::types::statics::{CrewAction, CrewContributionKind};

#[test]
fn planar_nexus_is_every_nonbasic_land_type() {
    // CR 205.3i + CR 604.3: Planar Nexus — "This land is every nonbasic land
    // type." A self CDA adding the twelve nonbasic land types and no basic one.
    for line in [
        "~ is every nonbasic land type.",
        "This land is every nonbasic land type.",
    ] {
        let def = parse_static_line(line).unwrap();
        assert_eq!(def.mode, StaticMode::Continuous);
        assert_eq!(def.affected, Some(TargetFilter::SelfRef));
        assert!(def.characteristic_defining, "{line}: expected a CDA");
        let subtypes: Vec<&str> = def
            .modifications
            .iter()
            .map(|m| match m {
                ContinuousModification::AddSubtype { subtype } => subtype.as_str(),
                other => panic!("{line}: unexpected modification {other:?}"),
            })
            .collect();
        assert_eq!(
            subtypes,
            vec![
                "Cave",
                "Desert",
                "Gate",
                "Lair",
                "Locus",
                "Mine",
                "Planet",
                "Power-Plant",
                "Sphere",
                "Tower",
                "Town",
                "Urza's",
            ],
            "{line}"
        );
    }
}

#[test]
fn medial_grant_conjuncts_of_a_comma_list_are_all_granted() {
    // CR 613.1f: every grant conjunct of a comma list is kept, not only the one
    // after the last "and gains" (Wings of Velis Vel).
    let mods = parse_continuous_modifications(
        "have base power and toughness 4/4, gains all creature types, and gains flying",
    );
    for expected in [
        ContinuousModification::SetPower { value: 4 },
        ContinuousModification::SetToughness { value: 4 },
        ContinuousModification::AddKeyword {
            keyword: Keyword::Changeling,
        },
        ContinuousModification::AddKeyword {
            keyword: Keyword::Flying,
        },
    ] {
        assert!(mods.contains(&expected), "missing {expected:?} in {mods:?}");
    }
    // A keyword the last conjunct already granted is not granted twice.
    let mods = parse_continuous_modifications("gets +1/+1, has flying, and has trample");
    let count = |kw: Keyword| {
        mods.iter()
            .filter(|m| {
                **m == ContinuousModification::AddKeyword {
                    keyword: kw.clone(),
                }
            })
            .count()
    };
    assert_eq!(count(Keyword::Flying), 1, "{mods:?}");
    assert_eq!(count(Keyword::Trample), 1, "{mods:?}");
}

#[test]
fn static_historic_creatures_have_flying() {
    // CR 700.6: "Historic creatures you control have flying." Historic is a
    // property (legendary, artifact or Saga), not a creature subtype.
    let def = parse_static_line("Historic creatures you control have flying.").unwrap();
    assert_eq!(def.mode, StaticMode::Continuous);
    assert_eq!(
        def.affected,
        Some(TargetFilter::Typed(
            TypedFilter::creature()
                .controller(ControllerRef::You)
                .properties(vec![FilterProp::Historic]),
        ))
    );
}

#[test]
fn crew_contribution_tripled_power_covers_station_and_crew() {
    let defs = parse_static_line_multi(
        "~ stations Spacecraft and crews Vehicles as though his power were tripled.",
    );
    assert_eq!(defs.len(), 1);
    assert_eq!(
        defs[0].mode,
        StaticMode::CrewContribution {
            kind: CrewContributionKind::PowerMultiplier { factor: 3 },
            actions: vec![CrewAction::Crew, CrewAction::Station],
        }
    );
}

/// CR 611.3a + CR 613.1f: "they" in a following sentence names the preceding
/// sentence's plural subject (Radiant Destiny): the city's-blessing gate grants
/// vigilance to creatures we control of the chosen type, not to the source.
#[test]
fn plural_pronoun_continuation_binds_previous_subject() {
    let defs = parse_static_line_multi(
        "Creatures you control of the chosen type get +1/+1. As long as you have the city's blessing, they also have vigilance.",
    );
    assert_eq!(defs.len(), 2, "{defs:?}");
    let subject = defs[0].affected.clone().expect("anthem subject");
    assert!(matches!(subject, TargetFilter::Typed(_)), "{subject:?}");
    assert_eq!(defs[1].affected.as_ref(), Some(&subject));
    assert_eq!(defs[1].condition, Some(StaticCondition::HasCityBlessing));
    assert_eq!(
        defs[1].modifications,
        vec![ContinuousModification::AddKeyword {
            keyword: Keyword::Vigilance
        }]
    );
    assert_eq!(
        defs[1].description.as_deref(),
        Some("As long as you have the city's blessing, they also have vigilance.")
    );

    // The plain form without a gate binds the same way.
    let defs = parse_static_line_multi("Other Elves you control get +1/+1. They have trample.");
    assert_eq!(defs.len(), 2, "{defs:?}");
    assert_eq!(defs[1].affected, defs[0].affected);
    assert_eq!(
        defs[1].modifications,
        vec![ContinuousModification::AddKeyword {
            keyword: Keyword::Trample
        }]
    );
}
