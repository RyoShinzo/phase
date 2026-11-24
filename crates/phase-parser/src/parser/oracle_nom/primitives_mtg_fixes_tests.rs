//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `primitives.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;

/// The or-closed list grammar reads an Oxford-comma list closed by "or",
/// and rolls an unclosed comma run back to its first leg, leaving the
/// comma for the caller.
#[test]
fn or_closed_list_rolls_back_unclosed_comma_run() {
    let mut list = parse_or_closed_list(parse_number);
    assert_eq!(list("1, 2, or 3"), Ok(("", vec![1, 2, 3])));
    assert_eq!(list("1 or 2 tokens"), Ok((" tokens", vec![1, 2])));
    assert_eq!(list("1, 2, 3 tokens"), Ok((", 2, 3 tokens", vec![1])));
    assert_eq!(
        list("1, 2, or 3, 4 tokens"),
        Ok((", 4 tokens", vec![1, 2, 3]))
    );
    assert!(list("tokens").is_err());
}
