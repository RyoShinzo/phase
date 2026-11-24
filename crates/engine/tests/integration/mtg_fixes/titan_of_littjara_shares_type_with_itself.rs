//! "Shares a creature type with it" in an untargeted self trigger names the source.
//!
//! Repro (Titan of Littjara): "Whenever this creature enters or attacks, you may draw a
//! card for each other creature you control that shares a creature type with it. If you
//! do, discard a card." The count parses as `ObjectCount` over creatures sharing a type
//! with `ParentTarget`; the trigger chooses no target, so the reference found nothing and
//! the Titan drew no card.
//!
//! CR 608.2c: in an ability that chose no object, "it" is the ability's source, the
//! fallback `targeting::resolved_targets` already gives an untargeted `ParentTarget`.

use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const TITAN: &str = "As this creature enters, choose a creature type.\nThis creature is the chosen type in addition to its other types.\nWhenever this creature enters or attacks, you may draw a card for each other creature you control that shares a creature type with it. If you do, discard a card.";

/// Two Faeries and a Bear on our side, the Titan in hand, six Islands, a library of five.
fn titan_game() -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    for name in ["Faerie One", "Faerie Two"] {
        scenario
            .add_creature(P0, name, 1, 1)
            .with_subtypes(vec!["Faerie"]);
    }
    scenario
        .add_creature(P0, "Bear", 2, 2)
        .with_subtypes(vec!["Bear"]);
    for _ in 0..6 {
        scenario.add_basic_land(P0, ManaColor::Blue);
    }
    for _ in 0..5 {
        scenario.add_card_to_library_top(P0, "Library Card");
    }
    let titan = scenario
        .add_creature_to_hand(P0, "Titan of Littjara", 6, 6)
        .with_subtypes(vec!["Illusion"])
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Blue, ManaCostShard::Blue],
            generic: 4,
        })
        .from_oracle_text(TITAN)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().all_creature_types = ["Bear", "Faerie", "Illusion"]
        .into_iter()
        .map(String::from)
        .collect();
    (runner, titan)
}

/// Casts the Titan naming `chosen`, accepts the draw and discards the first card in
/// hand when asked; returns (cards drawn, cards in the graveyard).
fn cast_naming(runner: &mut GameRunner, titan: ObjectId, chosen: &str) -> (usize, usize) {
    let card_id = runner.state().objects[&titan].card_id;
    let library_before = runner.state().players[0].library.len();
    runner
        .act(GameAction::CastSpell {
            object_id: titan,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("the Titan must be castable");
    for _ in 0..40 {
        match runner.state().waiting_for.clone() {
            WaitingFor::NamedChoice { .. } => {
                runner
                    .act(GameAction::ChooseOption {
                        choice: chosen.to_string(),
                    })
                    .expect("the creature type");
            }
            WaitingFor::OptionalEffectChoice { .. } => {
                runner
                    .act(GameAction::DecideOptionalEffect { accept: true })
                    .expect("accept the draw");
            }
            WaitingFor::DiscardChoice { .. } => {
                let pick = runner.state().players[0].hand[0];
                runner
                    .act(GameAction::SelectCards { cards: vec![pick] })
                    .expect("discard");
            }
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => break,
            WaitingFor::Priority { .. } | WaitingFor::ManaPayment { .. } => {
                runner.act(GameAction::PassPriority).expect("pass");
            }
            other => panic!("unexpected prompt: {other:?}"),
        }
    }
    assert_eq!(runner.state().objects[&titan].zone, Zone::Battlefield);
    let drawn = library_before - runner.state().players[0].library.len();
    (drawn, runner.state().players[0].graveyard.len())
}

#[test]
fn draws_for_each_other_creature_of_the_chosen_type() {
    let (mut runner, titan) = titan_game();
    assert_eq!(cast_naming(&mut runner, titan, "Faerie"), (2, 1));
}

#[test]
fn shares_its_printed_type_too() {
    // Naming Bear: the Titan is an Illusion Bear, so only the Bear shares a type.
    let (mut runner, titan) = titan_game();
    assert_eq!(cast_naming(&mut runner, titan, "Bear"), (1, 1));
}
