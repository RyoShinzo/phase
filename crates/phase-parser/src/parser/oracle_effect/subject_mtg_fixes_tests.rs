//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `subject.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;

#[test]
fn starts_with_subject_prefix_owner_or_controller_of() {
    assert!(starts_with_subject_prefix(
        "the owner of up to one target creature draws two cards"
    ));
    assert!(starts_with_subject_prefix(
        "the controller of target creature loses 2 life"
    ));
}
