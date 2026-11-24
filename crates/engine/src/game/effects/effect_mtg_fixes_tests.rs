//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `effect.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::ability::{ContinuousModification, ControllerRef, TargetRef};
use crate::types::keywords::{Keyword, ProtectionTarget};
use crate::types::player::PlayerId;

/// CR 702.16k + CR 608.2c + CR 608.2h: "you gain protection from that
/// player" is fixed to the declared player target when the effect is
/// applied; with no player target (or only an object target) the payload is
/// left unchanged and matches nothing (CR 609.3).
#[test]
fn player_protection_from_that_player_latches_the_declared_target() {
    let state = GameState::new_two_player(42);
    let modifications = [ContinuousModification::AddKeyword {
        keyword: Keyword::Protection(ProtectionTarget::FromPlayer(ControllerRef::TargetPlayer)),
    }];

    let mut ability = ResolvedAbility::new(
        Effect::NoOp,
        vec![TargetRef::Player(PlayerId(1))],
        ObjectId(100),
        PlayerId(0),
    );
    let latched = snapshot_transient_modifications(&state, &ability, &modifications);
    assert!(matches!(
        latched[0],
        ContinuousModification::AddKeyword {
            keyword: Keyword::Protection(ProtectionTarget::FromPlayer(
                ControllerRef::SpecificPlayer { id: PlayerId(1) }
            ))
        }
    ));

    ability.targets = vec![TargetRef::Object(ObjectId(7))];
    let unchanged = snapshot_transient_modifications(&state, &ability, &modifications);
    assert!(matches!(
        unchanged[0],
        ContinuousModification::AddKeyword {
            keyword: Keyword::Protection(ProtectionTarget::FromPlayer(ControllerRef::TargetPlayer))
        }
    ));
}
