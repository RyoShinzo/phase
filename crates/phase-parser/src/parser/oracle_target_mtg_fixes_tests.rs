//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `oracle_target.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::parser::oracle_ir::context::ParseContext;

/// CR 202.3: Kozilek's Unsealing — "with mana value 4, 5, or 6" is an
/// exact-value list, one EQ comparison per value, consumed up to (not
/// including) the clause comma.
#[test]
fn creature_spell_with_mana_value_4_5_or_6() {
    let mut ctx = ParseContext::default();
    let input =
        "with mana value 4, 5, or 6, create two 0/1 colorless eldrazi spawn creature tokens";
    let (prop, consumed) =
        parse_mana_value_suffix(input, &mut ctx).expect("mana value list parses");
    assert_eq!(
        &input[consumed..],
        ", create two 0/1 colorless eldrazi spawn creature tokens"
    );
    let mv_eq = |value| FilterProp::Cmc {
        comparator: Comparator::EQ,
        value: QuantityExpr::Fixed { value },
    };
    assert_eq!(
        prop,
        FilterProp::AnyOf {
            props: vec![mv_eq(4), mv_eq(5), mv_eq(6)],
        }
    );

    // Firemind's Foresight: a single number before a clause comma stays an
    // exact EQ reading and leaves the comma.
    let input = "with mana value 3, reveal it, and put it into your hand";
    let (prop, consumed) =
        parse_mana_value_suffix(input, &mut ctx).expect("single mana value parses");
    assert_eq!(&input[consumed..], ", reveal it, and put it into your hand");
    assert_eq!(prop, mv_eq(3));

    // An "or" tail that is neither a comparator nor a number leg declines
    // the whole suffix (the pre-list behaviour), instead of keeping EQ 4 and
    // leaving "or blue" behind. The two positives above are the reach-guard.
    assert_eq!(
        parse_mana_value_suffix("with mana value 4 or blue", &mut ctx),
        None
    );
}
