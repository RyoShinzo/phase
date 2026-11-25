//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `condition.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them.

use super::*;
use crate::types::ability::{
    Comparator, FilterProp, PlayerScope, QuantityExpr, QuantityRef, StaticCondition,
    TargetFilter, TurnWindow, TypeFilter,
};

/// The `BattlefieldEntriesThisTurn` tally of a condition, with its comparator
/// and threshold.
fn entry_tally(text: &str) -> (PlayerScope, TargetFilter, TurnWindow, Comparator, i32) {
    let (rest, condition) = parse_inner_condition(text).unwrap();
    assert_eq!(rest, "", "{text}");
    match condition {
        StaticCondition::QuantityComparison {
            lhs:
                QuantityExpr::Ref {
                    qty:
                        QuantityRef::BattlefieldEntriesThisTurn {
                            player,
                            filter,
                            window,
                        },
                },
            comparator,
            rhs: QuantityExpr::Fixed { value },
        } => (player, filter, window, comparator, value),
        other => panic!("expected a battlefield-entry tally for {text:?}, got {other:?}"),
    }
}

/// CR 403.3 + CR 608.2i: Ephara, God of the Polis — "you had another creature
/// enter the battlefield under your control last turn" reads the previous
/// turn's entries of another creature under the controller's control.
#[test]
fn you_had_another_creature_enter_last_turn() {
    let (player, filter, window, comparator, value) = entry_tally(
        "you had another creature enter the battlefield under your control last turn",
    );
    assert_eq!(player, PlayerScope::Controller);
    assert_eq!(window, TurnWindow::LastTurn);
    assert_eq!((comparator, value), (Comparator::GE, 1));
    let TargetFilter::Typed(filter) = filter else {
        panic!("expected a typed filter, got {filter:?}");
    };
    assert_eq!(filter.controller, None);
    assert!(filter.type_filters.contains(&TypeFilter::Creature));
    assert!(filter.properties.contains(&FilterProp::Another));
}

/// The turn word is one axis over every entered-under-control surface: the
/// counted and singular forms, "you had … enter" and "… entered", this turn and
/// last turn.
#[test]
fn the_turn_word_selects_the_window() {
    for (text, expected_window, expected_value) in [
        (
            "you had another creature enter the battlefield under your control this turn",
            TurnWindow::ThisTurn,
            1,
        ),
        (
            "you had two or more creatures enter the battlefield under your control last turn",
            TurnWindow::LastTurn,
            2,
        ),
        (
            "a land entered the battlefield under your control last turn",
            TurnWindow::LastTurn,
            1,
        ),
        (
            "two or more creatures entered the battlefield under your control this turn",
            TurnWindow::ThisTurn,
            2,
        ),
    ] {
        let (_, _, window, comparator, value) = entry_tally(text);
        assert_eq!(window, expected_window, "{text}");
        assert_eq!((comparator, value), (Comparator::GE, expected_value), "{text}");
    }
}

/// "during their last turn" names another window (the opponent's own last
/// turn, Simic, Value Engine) and is not read as the previous turn.
#[test]
fn during_their_last_turn_is_not_the_previous_turn() {
    assert!(parse_inner_condition(
        "an opponent had a land enter the battlefield under their control during their last turn"
    )
    .map_or(true, |(rest, _)| !rest.is_empty()));
}
