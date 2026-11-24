//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `keywords.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;

/// CR 702.16k + CR 608.2c: "protection from that player" names the player
/// targeted by the enclosing ability, not a card type. The atomic-quality
/// classifier emits the typed player reference the resolver fixes to the
/// declared target; the neighbouring anaphors keep their own variants.
#[test]
fn parse_protection_target_player_anaphors() {
    assert_eq!(
        parse_protection_target("that player"),
        ProtectionTarget::FromPlayer(super::super::ability::ControllerRef::TargetPlayer),
    );
    assert_eq!(
        parse_protection_target("the chosen player"),
        ProtectionTarget::ChosenPlayer,
    );
    assert_eq!(
        parse_protection_target("each of your opponents"),
        ProtectionTarget::FromPlayer(super::super::ability::ControllerRef::Opponent),
    );
    assert_eq!(
        parse_protection_target("red"),
        ProtectionTarget::Color(ManaColor::Red),
    );
}
