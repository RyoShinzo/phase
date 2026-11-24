//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `filter.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;

/// CR 208.1: Sarkhan's Unsealing — "power 4, 5, or 6" is an exact-value
/// list, one EQ comparison per value, and the clause comma after it stays
/// in the remainder.
#[test]
fn pt_comparison_value_list_power() {
    let (rest, p) =
        parse_pt_comparison("power 4, 5, or 6, this enchantment deals 4 damage to any target.")
            .unwrap();
    assert_eq!(rest, ", this enchantment deals 4 damage to any target.");
    let power_eq = |value| FilterProp::PtComparison {
        stat: PtStat::Power,
        scope: PtValueScope::Current,
        comparator: Comparator::EQ,
        value: QuantityExpr::Fixed { value },
    };
    assert_eq!(
        p,
        FilterProp::AnyOf {
            props: vec![power_eq(4), power_eq(5), power_eq(6)],
        }
    );

    // Arrogant Bloodlord: a comparator tail before a clause comma is not a
    // list and keeps its LE reading and the comma.
    let (rest, p) =
        parse_pt_comparison("power 1 or less, destroy this creature at end of combat.").unwrap();
    assert_eq!(rest, ", destroy this creature at end of combat.");
    assert_eq!(
        p,
        FilterProp::PtComparison {
            stat: PtStat::Power,
            scope: PtValueScope::Current,
            comparator: Comparator::LE,
            value: QuantityExpr::Fixed { value: 1 },
        }
    );
}
