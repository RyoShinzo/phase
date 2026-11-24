//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `keywords.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::ability::ControllerRef;
use crate::types::identifiers::{CardId, ObjectId};
use crate::types::keywords::HexproofFilter;
use crate::types::mana::ManaColor;
use crate::types::player::PlayerId;
use crate::types::zones::Zone;

fn make_obj() -> GameObject {
    GameObject::new(
        ObjectId(1),
        CardId(1),
        PlayerId(0),
        "Test".to_string(),
        Zone::Battlefield,
    )
}

/// CR 702.16k: "protection from that player" is the player bearer's
/// `FromPlayer(SpecificPlayer)` arm; the source matches when that player
/// controls (or, off the battlefield, owns) it.
#[test]
fn player_bearer_from_player_specific_player_matches_controller_or_owner() {
    let mut owned_source = make_obj();
    owned_source.zone = Zone::Graveyard;
    owned_source.controller = PlayerId(0);
    owned_source.owner = PlayerId(1);

    let protection =
        ProtectionTarget::FromPlayer(ControllerRef::SpecificPlayer { id: PlayerId(1) });
    assert!(
        source_matches_protection_target_for_player(&protection, PlayerId(0), &owned_source),
        "CR 702.16k: a source owned by the named player matches"
    );
    let mut other = owned_source.clone();
    other.owner = PlayerId(0);
    assert!(
        !source_matches_protection_target_for_player(&protection, PlayerId(0), &other),
        "a source neither controlled nor owned by the named player does not match"
    );
}

/// CR 702.16: the player bearer has no chosen attributes, so the chosen
/// quality arms are inert for it; a concrete color still matches.
#[test]
fn player_bearer_reads_concrete_colors_and_ignores_chosen_attributes() {
    let mut red_source = make_obj();
    red_source.color = vec![ManaColor::Red];

    assert!(source_matches_protection_target_for_player(
        &ProtectionTarget::Color(ManaColor::Red),
        PlayerId(0),
        &red_source,
    ));
    assert!(!source_matches_protection_target_for_player(
        &ProtectionTarget::Color(ManaColor::Blue),
        PlayerId(0),
        &red_source,
    ));
    assert!(!source_matches_protection_target_for_player(
        &ProtectionTarget::ChosenColor,
        PlayerId(0),
        &red_source,
    ));
}

/// CR 702.11d: the hexproof-from filter matcher is bearer-independent, so
/// the player path reuses it.
#[test]
fn hexproof_filter_matches_concrete_quality_for_any_bearer() {
    let mut blue_source = make_obj();
    blue_source.color = vec![ManaColor::Blue];

    assert!(source_matches_hexproof_filter(
        &HexproofFilter::Color(ManaColor::Blue),
        &blue_source,
    ));
    assert!(!source_matches_hexproof_filter(
        &HexproofFilter::Color(ManaColor::Black),
        &blue_source,
    ));
}
