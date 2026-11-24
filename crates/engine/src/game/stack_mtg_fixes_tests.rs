//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `stack.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use super::resolve_top;
use crate::game::layers::{evaluate_layers, flush_layers};
use crate::game::zones::create_object;
use crate::types::ability::QuantityRef;
use crate::types::ability::{QuantityExpr, TargetFilter, TypeFilter, TypedFilter};
use crate::types::card_type::CoreType;
use crate::types::game_state::{GameState, StackEntry, StackEntryKind};
use crate::types::identifiers::{CardId, ObjectId};
use crate::types::player::PlayerId;
use crate::types::zones::Zone;
use std::sync::Arc;

/// Assert every battlefield object's computed power/toughness/loyalty and
/// keyword set are identical across two states.
fn assert_pt_identical(a: &GameState, b: &GameState, label: &str) {
    assert_eq!(
        a.battlefield.len(),
        b.battlefield.len(),
        "{label}: battlefield size mismatch"
    );
    for &id in a.battlefield.iter() {
        let oa = a.objects.get(&id).expect("a object");
        let ob = b.objects.get(&id).expect("b object");
        assert_eq!(oa.power, ob.power, "{label}: power mismatch for {id:?}");
        assert_eq!(
            oa.toughness, ob.toughness,
            "{label}: toughness mismatch for {id:?}"
        );
        assert_eq!(
            oa.keywords, ob.keywords,
            "{label}: keyword mismatch for {id:?}"
        );
    }
}

/// A 1/1 creature of one creature type, on the battlefield, not yet
/// marked as entered.
fn typed_creature(state: &mut GameState, card_id: u64, subtype: &str) -> ObjectId {
    let id = create_object(
        state,
        CardId(card_id),
        PlayerId(0),
        format!("{subtype} {card_id}"),
        Zone::Battlefield,
    );
    let o = state.objects.get_mut(&id).unwrap();
    o.base_power = Some(1);
    o.base_toughness = Some(1);
    o.power = Some(1);
    o.toughness = Some(1);
    o.base_card_types.core_types = vec![CoreType::Creature];
    o.base_card_types.subtypes = vec![subtype.to_string()];
    o.card_types = o.base_card_types.clone();
    id
}

/// ISOLATED single-flush measurement of the entry-aware escalation
/// decision. `setup_board` returns a board with the anthem already in
/// place (still `Full`-dirty). The helper:
///   1. flushes the board to Clean (the anthem's initial full pass — NOT
///      measured),
///   2. invokes `add_entry` to create the entering object and returns its
///      id (the closure must `mark_layers_entered` so the dirty lattice is
///      `EnteredObjects`),
///   3. resets the counter and performs a SINGLE `flush_layers`, capturing
///      exactly the entry-aware escalation decision (0 = incremental fast
///      path engaged, >=1 = escalated to a full pass),
///   4. builds a forced-Full reference from the same post-entry board for
///      dual-run characteristic identity.
///
/// This isolates the escalation DECISION from the token-RESOLUTION
/// pipeline (which does unrelated full passes during `Effect::Token`
/// resolution / SBA).
///
/// The escalation signal is read RACE-FREE from
/// `incremental_flush_must_escalate` directly (a pure predicate over the
/// post-entry board) rather than from the process-wide
/// `FULL_EVALUATE_LAYERS_COUNT`, which `cargo test`'s parallel runner
/// would otherwise corrupt. Returns `(incremental_board, escalated,
/// forced_full_board)`, where `escalated == false` means the entry-aware
/// fast path engaged and `escalated == true` means the entry forced a full
/// pass. The dual-run identity (`incremental_board` vs `forced_full_board`)
/// is the under-escalation tripwire regardless of the decision.
fn flush_entry_and_forced(
    setup_board: impl Fn() -> GameState,
    add_entry: impl Fn(&mut GameState) -> ObjectId,
) -> (GameState, bool, GameState) {
    // Normal path: flush the anthem in, add the entry, read the decision,
    // then flush incrementally (or full, per the decision).
    let mut normal = setup_board();
    flush_layers(&mut normal);
    add_entry(&mut normal);
    let entered_ids: std::collections::BTreeSet<ObjectId> = match &normal.layers_dirty {
        crate::types::game_state::LayersDirty::EnteredObjects(ids) => ids.clone(),
        other => panic!("expected EnteredObjects dirty state, got {other:?}"),
    };
    let escalated = crate::game::layers::incremental_flush_must_escalate(&normal, &entered_ids);
    flush_layers(&mut normal);

    // Forced-Full reference: same board + entry, then a full re-eval.
    let mut forced = setup_board();
    flush_layers(&mut forced);
    add_entry(&mut forced);
    forced.layers_dirty = crate::types::game_state::LayersDirty::Full;
    evaluate_layers(&mut forced);
    (normal, escalated, forced)
}

fn setup() -> GameState {
    GameState::new_two_player(42)
}

/// Build a Coat of Arms board: "Each creature gets +1/+1 for each other
/// creature on the battlefield that shares at least one creature type
/// with it", over two Faerie 1/1s. The count's
/// `SharesQuality { reference: ParentTarget }` binds to each recipient
/// (CR 613.4c), so whether an entrant joins a recipient's count depends
/// on that recipient's creature types, and the probe without a
/// recipient can never see it.
fn shared_type_count_anthem_board() -> GameState {
    use crate::types::ability::{
        ContinuousModification, FilterProp, SharedQuality, SharedQualityRelation, StaticDefinition,
    };
    use crate::types::statics::StaticMode;
    let mut state = setup();
    state.all_creature_types = vec!["Cat".to_string(), "Faerie".to_string()];
    for card_id in [380, 381] {
        typed_creature(&mut state, card_id, "Faerie");
    }
    let coat = create_object(
        &mut state,
        CardId(390),
        PlayerId(0),
        "Coat of Arms".to_string(),
        Zone::Battlefield,
    );
    let sharers = QuantityExpr::Ref {
        qty: QuantityRef::ObjectCount {
            filter: TargetFilter::Typed(TypedFilter {
                type_filters: vec![TypeFilter::Creature],
                properties: vec![
                    FilterProp::Another,
                    FilterProp::SharesQuality {
                        quality: SharedQuality::CreatureType,
                        reference: Some(Box::new(TargetFilter::ParentTarget)),
                        relation: SharedQualityRelation::Shares,
                    },
                ],
                ..Default::default()
            }),
        },
    };
    let mut sd = StaticDefinition::new(StaticMode::Continuous);
    sd.affected = Some(TargetFilter::Typed(TypedFilter::new(TypeFilter::Creature)));
    sd.modifications = vec![
        ContinuousModification::AddDynamicPower {
            value: sharers.clone(),
        },
        ContinuousModification::AddDynamicToughness { value: sharers },
    ];
    {
        let o = state.objects.get_mut(&coat).unwrap();
        o.base_static_definitions = Arc::new(vec![sd.clone()]);
        o.static_definitions = vec![sd].into();
        o.base_card_types.core_types = vec![CoreType::Artifact];
        o.card_types.core_types = vec![CoreType::Artifact];
    }
    state.layers_dirty = crate::types::game_state::LayersDirty::Full;
    state
}

#[test]
fn warp_granted_only_in_hand_still_exiles() {
    // CR 702.185a: a card that had warp only in hand (Tannuk, Steadfast Second)
    // has no warp keyword on the battlefield; its paid warp cost still exiles it.
    let mut state = setup();
    state.turn_number = 3;
    let obj_id = create_object(
        &mut state,
        CardId(1),
        PlayerId(0),
        "Granted Warp Ship".to_string(),
        Zone::Battlefield,
    );
    state
        .objects
        .get_mut(&obj_id)
        .unwrap()
        .card_types
        .core_types
        .push(CoreType::Artifact);
    state.stack.push_back(StackEntry {
        id: obj_id,
        source_id: obj_id,
        controller: PlayerId(0),
        kind: StackEntryKind::Spell {
            card_id: CardId(1),
            ability: None,
            casting_variant: crate::types::game_state::CastingVariant::Warp,
            actual_mana_spent: 0,
        },
    });
    let mut events = Vec::new();
    resolve_top(&mut state, &mut events);
    assert_eq!(
        state.delayed_triggers.len(),
        1,
        "a paid warp cost creates the end-step exile without a warp keyword"
    );
}

/// CR 611.3a + CR 613.4c: a third Faerie entering joins the shared-type
/// count of each Faerie already there, so the flush must escalate: each
/// Faerie is 3/3, the pre-existing ones included.
#[test]
fn shared_type_count_entry_escalates_and_matches_full() {
    let (normal, escalated, forced) = flush_entry_and_forced(shared_type_count_anthem_board, |s| {
        let id = typed_creature(s, 382, "Faerie");
        crate::game::layers::mark_layers_entered(s, id);
        id
    });
    assert!(
        escalated,
        "a Faerie entry joins each pre-existing Faerie's shared-type count"
    );
    assert_pt_identical(&normal, &forced, "shared-type count Faerie entry");
    let faeries: Vec<_> = normal
        .battlefield
        .iter()
        .filter_map(|id| normal.objects.get(id))
        .filter(|o| o.card_types.subtypes.iter().any(|t| t == "Faerie"))
        .map(|o| (o.power, o.toughness))
        .collect();
    assert_eq!(faeries, vec![(Some(3), Some(3)); 3]);
}

/// A Cat entering shares no creature type with the Faeries, so it joins
/// no recipient's count and the flush stays incremental.
#[test]
fn shared_type_count_unrelated_entry_does_not_escalate_and_matches_full() {
    let (normal, escalated, forced) = flush_entry_and_forced(shared_type_count_anthem_board, |s| {
        let id = typed_creature(s, 383, "Cat");
        crate::game::layers::mark_layers_entered(s, id);
        id
    });
    assert!(!escalated, "a Cat joins no Faerie's shared-type count");
    assert_pt_identical(&normal, &forced, "shared-type count Cat entry");
}
