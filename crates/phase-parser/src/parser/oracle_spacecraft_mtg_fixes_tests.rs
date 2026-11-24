//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `oracle_spacecraft.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;

/// u4-c2 test shim: map the source-line-tagged preprocessor return back to the
/// bare-def shape the existing assertions read.
#[allow(clippy::type_complexity)]
fn spacecraft_test_lines(
    lines: &[&str],
    name: &str,
    base_trigger_index: PrintedTriggerIndex,
) -> (
    Vec<StaticDefinition>,
    Vec<TriggerDefinition>,
    Vec<AbilityDefinition>,
    Vec<usize>,
) {
    let (statics, triggers, abilities, consumed) =
        parse_spacecraft_threshold_lines(lines, name, base_trigger_index);
    (
        statics.into_iter().map(|(_, s)| s.definition).collect(),
        triggers.into_iter().map(|(_, t)| t).collect(),
        abilities.into_iter().map(|(_, a)| a).collect(),
        consumed,
    )
}

#[test]
fn creature_threshold_follows_reminder_then_highest_symbol() {
    let prometheus = [
        "Station (Station only as a sorcery. It's an artifact creature at 4+.)",
        "4+ | Flying",
        "8+ | Multivector Assault Mode — Whenever ~ deals combat damage to an opponent, it deals that much damage to each other opponent.",
    ];
    assert_eq!(spacecraft_creature_threshold(&prometheus), Some(4));
    let reminderless = ["Station", "2+ | Vigilance", "8+ | Flying"];
    assert_eq!(spacecraft_creature_threshold(&reminderless), Some(8));
    assert_eq!(spacecraft_creature_threshold(&["Station"]), None);
}

#[test]
fn ability_word_trigger_keeps_station_gate() {
    let lines = [
        "8+ | Multivector Assault Mode — Whenever ~ deals combat damage to an opponent, it deals that much damage to each other opponent.",
    ];
    let (_, triggers, _, consumed) =
        spacecraft_test_lines(&lines, "Test", PrintedTriggerIndex::from_slot_for_test(0));
    assert_eq!(consumed, vec![0]);
    assert_eq!(triggers.len(), 1);
    assert!(matches!(
        triggers[0].condition,
        Some(TriggerCondition::HasCounters { minimum: 8, .. })
    ));
}
