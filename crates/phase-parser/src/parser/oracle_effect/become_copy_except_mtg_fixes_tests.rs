//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `become_copy_except.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::types::keywords::Keyword;

/// CR 707.9a: "the token has flying" is the same keyword grant as "it has
/// flying" (Irenicus's Vile Duplication); the flying used to be dropped.
#[test]
fn irenicuss_vile_duplication_token_has_flying_and_is_not_legendary() {
    let (_, mods) = parse_except_clause(
        ", except the token has flying and it isn't legendary.",
        "Irenicus's Vile Duplication",
        &ParseContext::default(),
    )
    .unwrap();
    assert!(
        mods.contains(&ContinuousModification::AddKeyword {
            keyword: Keyword::Flying,
        }),
        "the token must get flying; got {mods:?}"
    );
    assert!(
        mods.iter()
            .any(|m| matches!(m, ContinuousModification::RemoveSupertype { .. })),
        "the token must not be legendary; got {mods:?}"
    );
}
