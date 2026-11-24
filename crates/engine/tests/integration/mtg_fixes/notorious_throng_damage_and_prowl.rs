//! Notorious Throng counts all damage dealt to the opponents this turn, and its prowl
//! cost grants the extra turn.
//!
//! Oracle: "Prowl {5}{U} (You may cast this for its prowl cost if you dealt combat damage
//! to a player this turn with a Rogue.)\nCreate X 1/1 black Faerie Rogue creature tokens
//! with flying, where X is the damage dealt to your opponents this turn. If this spell's
//! prowl cost was paid, take an extra turn after this one."
//!
//! Two misreads: the quantity was unparsed, so no token was made; and a prowl spell was
//! stamped as prowl-paid only when a permanent entered, so the resolving sorcery never
//! saw its prowl cost paid and took no extra turn.
//!
//! CR 120.1 + CR 120.9 (its ruling: all damage by all sources to the opponents),
//! CR 702.76a, CR 500.7.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::ability::TargetRef;
use engine::types::actions::{AlternativeCastDecision, GameAction};
use engine::types::game_state::{CastPaymentMode, DamageRecord, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaCost, ManaCostShard, ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const THRONG: &str = "Prowl {5}{U} (You may cast this for its prowl cost if you dealt combat damage to a player this turn with a Rogue.)\nCreate X 1/1 black Faerie Rogue creature tokens with flying, where X is the damage dealt to your opponents this turn. If this spell's prowl cost was paid, take an extra turn after this one.";

fn damage(target: PlayerId, amount: u32, is_combat: bool) -> DamageRecord {
    DamageRecord {
        target: TargetRef::Player(target),
        target_controller: target,
        amount,
        is_combat,
        ..Default::default()
    }
}

/// The Throng in hand with six blue mana; this turn 2 combat and 3 noncombat damage to
/// the opponent and 3 to us. With `rogue_hit`, a Rogue of ours dealt the combat damage.
fn throng_game(rogue_hit: bool) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PostCombatMain);
    let throng = scenario
        .add_spell_to_hand_from_oracle(P0, "Notorious Throng", false, THRONG)
        .with_subtypes(vec!["Rogue"])
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Blue],
            generic: 3,
        })
        .id();
    let pool = (0..6)
        .map(|_| ManaUnit::new(ManaType::Blue, ObjectId(0), false, vec![]))
        .collect();
    scenario.with_mana_pool(P0, pool);
    let mut runner = scenario.build();
    let state = runner.state_mut();
    state.active_player = P0;
    for record in [
        damage(P1, 2, true),
        damage(P1, 3, false),
        damage(P0, 3, false),
    ] {
        state.damage_dealt_this_turn.push_back(record);
    }
    if rogue_hit {
        state
            .creature_types_dealt_combat_damage_this_turn
            .insert((P0, "Rogue".to_string()));
    }
    (runner, throng)
}

/// Casts the Throng, for its prowl cost if `prowl`; returns (tokens, extra turns for us).
fn cast_throng(runner: &mut GameRunner, throng: ObjectId, prowl: bool) -> (usize, usize) {
    let card_id = runner.state().objects[&throng].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: throng,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("the Throng must be castable");
    for _ in 0..20 {
        match runner.state().waiting_for.clone() {
            WaitingFor::AlternativeCastChoice { .. } => {
                let choice = if prowl {
                    AlternativeCastDecision::Alternative
                } else {
                    AlternativeCastDecision::Normal
                };
                runner
                    .act(GameAction::ChooseAlternativeCast { choice })
                    .expect("cost choice");
            }
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => break,
            WaitingFor::Priority { .. } | WaitingFor::ManaPayment { .. } => {
                runner.act(GameAction::PassPriority).expect("pass");
            }
            other => panic!("unexpected prompt: {other:?}"),
        }
    }
    let state = runner.state();
    let tokens = state
        .battlefield
        .iter()
        .filter(|id| state.objects[id].is_token && state.objects[id].controller == P0)
        .count();
    let extra = state.extra_turns.iter().filter(|t| t.player == P0).count();
    (tokens, extra)
}

#[test]
fn counts_combat_and_noncombat_damage_to_opponents_only() {
    let (mut runner, throng) = throng_game(false);
    assert_eq!(cast_throng(&mut runner, throng, false), (5, 0));
}

#[test]
fn prowl_cost_paid_takes_an_extra_turn() {
    let (mut runner, throng) = throng_game(true);
    assert_eq!(cast_throng(&mut runner, throng, true), (5, 1));
}

#[test]
fn printed_cost_with_prowl_open_takes_no_extra_turn() {
    let (mut runner, throng) = throng_game(true);
    assert_eq!(cast_throng(&mut runner, throng, false), (5, 0));
}
