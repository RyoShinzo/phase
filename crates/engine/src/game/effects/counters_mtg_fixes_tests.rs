//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `counters.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::*;
use crate::game::zones::create_object;
use crate::types::identifiers::CardId;
use crate::types::player::PlayerId;
use crate::types::zones::Zone;

/// CR 614.1c + CR 122.6 + CR 614.5: a creature entering with counters of
/// one kind from several contributions (Mikaeus, the Lunarch's X counters
/// and riot's additional counter) gets them in one placement, so a
/// Hardened-Scales-class replacement adds one, not one per contribution,
/// and one `CounterAdded` event is emitted for the whole amount.
#[test]
fn etb_counters_of_one_kind_are_one_placement() {
    use crate::game::engine_replacement::apply_etb_counters;
    use crate::types::ability::{QuantityModification, ReplacementDefinition, TargetFilter};
    use crate::types::card_type::CoreType;
    use crate::types::replacements::ReplacementEvent;

    let mut state = GameState::new_two_player(42);
    let scales_id = create_object(
        &mut state,
        CardId(2),
        PlayerId(0),
        "Counter Adder".to_string(),
        Zone::Battlefield,
    );
    let mut scales = ReplacementDefinition::new(ReplacementEvent::AddCounter);
    scales.valid_card = Some(TargetFilter::Any);
    scales.quantity_modification = Some(QuantityModification::Plus { value: 1 });
    state
        .objects
        .get_mut(&scales_id)
        .unwrap()
        .replacement_definitions
        .push(scales);

    let creature = create_object(
        &mut state,
        CardId(1),
        PlayerId(0),
        "Entering Creature".to_string(),
        Zone::Battlefield,
    );
    state
        .objects
        .get_mut(&creature)
        .unwrap()
        .card_types
        .core_types
        .push(CoreType::Creature);

    let contributions = vec![
        (CounterType::Plus1Plus1, 2u32),
        (CounterType::Plus1Plus1, 1),
    ];
    let mut events = Vec::new();
    assert!(apply_etb_counters(
        &mut state,
        creature,
        &contributions,
        &mut events
    ));

    assert_eq!(
        state.objects[&creature]
            .counters
            .get(&CounterType::Plus1Plus1)
            .copied(),
        Some(4),
        "2 + 1 counters in one placement, plus one from the replacement"
    );
    let added: Vec<u32> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::CounterAdded {
                object_id, count, ..
            } if *object_id == creature => Some(*count),
            _ => None,
        })
        .collect();
    assert_eq!(
        added,
        vec![4],
        "one CounterAdded event for the whole placement"
    );
}
