//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `static_abilities.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::game::zones::create_object;
use crate::types::ability::{ControllerRef, StaticDefinition, TargetFilter, TypedFilter};
use crate::types::identifiers::CardId;
use crate::types::statics::StaticMode;
use crate::types::zones::Zone;

fn setup() -> GameState {
    GameState::new_two_player(42)
}

/// CR 702.16 + CR 702.16b: A `PlayerProtection(Color(_))` static (the
/// printed "You have protection from [quality]" class) is read through the
/// shared player matcher: the protected player is protected from sources of
/// that color and not from other sources.
#[test]
fn player_protection_static_color_delegates_to_the_shared_matcher() {
    let mut state = setup();
    let grantor = create_object(
        &mut state,
        CardId(1),
        PlayerId(0),
        "Protection Grantor".to_string(),
        Zone::Battlefield,
    );
    state
        .objects
        .get_mut(&grantor)
        .unwrap()
        .static_definitions
        .push(
            StaticDefinition::new(StaticMode::PlayerProtection(
                crate::types::keywords::ProtectionTarget::Color(crate::types::mana::ManaColor::Red),
            ))
            .affected(TargetFilter::Typed(
                TypedFilter::default().controller(ControllerRef::You),
            )),
        );

    let red_source = create_object(
        &mut state,
        CardId(2),
        PlayerId(1),
        "Red Source".to_string(),
        Zone::Battlefield,
    );
    state.objects.get_mut(&red_source).unwrap().color = vec![crate::types::mana::ManaColor::Red];
    let blue_source = create_object(
        &mut state,
        CardId(3),
        PlayerId(1),
        "Blue Source".to_string(),
        Zone::Battlefield,
    );
    state.objects.get_mut(&blue_source).unwrap().color = vec![crate::types::mana::ManaColor::Blue];

    assert!(
        player_protection_from(&state, PlayerId(0), Some(red_source)),
        "CR 702.16b: protection from red covers a red source"
    );
    assert!(
        !player_protection_from(&state, PlayerId(0), Some(blue_source)),
        "a blue source is outside protection from red"
    );
}
