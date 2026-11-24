//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `sequence.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;

/// Helper: extract just the text fields from split_clause_sequence output.
fn clause_texts(input: &str) -> Vec<String> {
    split_clause_sequence(input)
        .into_iter()
        .map(|c| c.text)
        .collect()
}

#[test]
fn quoted_grant_splits_before_token_goaded_followup() {
    // CR 701.15a + CR 701.15b: Nettling Nuisance — "The token is goaded for
    // the rest of the game." after the token's quoted ability is the goad
    // follow-up, a clause of its own; attached, the token parse dropped it.
    for followup in [
        "The token is goaded.",
        "The token is goaded for the rest of the game.",
    ] {
        let chunks = clause_texts(&format!(
            "that player creates a 4/2 red Pirate creature token with \"This token can't block.\" {followup}"
        ));
        assert_eq!(
            chunks.len(),
            2,
            "expected the goad follow-up split off: {chunks:?}"
        );
    }
}

/// CR 701.15a + CR 701.15b: the token keeps its quoted "can't block" and is
/// goaded for the rest of the game (Nettling Nuisance).
#[test]
fn token_with_quoted_ability_then_goaded_chain() {
    use super::super::parse_effect_chain;

    let def = parse_effect_chain(
        "that player creates a 4/2 red Pirate creature token with \"This token can't block.\" The token is goaded for the rest of the game.",
        AbilityKind::Spell,
    );
    assert!(
        matches!(def.effect.as_ref(), Effect::Token { static_abilities, .. } if static_abilities.iter().any(|s| s.mode == StaticMode::CantBlock)),
        "token with its quoted can't-block: {:?}",
        def.effect
    );
    assert_eq!(def.duration, None, "the goad's duration is not the token's");
    let sub = def.sub_ability.as_ref().expect("goad sub_ability");
    match sub.effect.as_ref() {
        Effect::GenericEffect {
            static_abilities,
            duration,
            target,
            ..
        } => {
            assert_eq!(*target, Some(TargetFilter::LastCreated));
            assert_eq!(*duration, Some(Duration::Permanent));
            assert!(static_abilities[0].modifications.iter().any(|m| matches!(
                m,
                ContinuousModification::AddStaticMode {
                    mode: StaticMode::Goaded
                }
            )));
        }
        other => panic!("expected GenericEffect goad sub, got {other:?}"),
    }
}
