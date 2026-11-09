//! A resolution-time payment keeps the resolving ability's "those cards".
//!
//! Repro (games 1 and 3 of a Betor, Ancestor's Voice list): Ripples of Undeath, "At the
//! beginning of your first main phase, mill three cards. Then you may pay {1} and 3 life.
//! If you do, put a card from among those cards into your hand." Paying took {1} and 3
//! life and put no card into the hand. The composite payment is staged on a shadow state
//! (`payment_transaction::begin`), which resolved the payment at depth 0: the top-level
//! prelude cleared the chain tracked set the mill had published, the payment then
//! published a new, empty one, and "those cards" read it.
//!
//! CR 608.2c: the payment is one instruction of the resolving ability, so "those cards"
//! still names the milled cards after it.

use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaColor;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const RIPPLES_OF_UNDEATH: &str = "At the beginning of your first main phase, mill three cards. Then you may pay {1} and 3 life. If you do, put a card from among those cards into your hand.";

/// Ripples of Undeath, two Swamps, and a library of a card to draw, three to mill and one
/// below them. Returns the runner in our untap step and the three cards to be milled.
fn ripples_game() -> (GameRunner, Vec<ObjectId>) {
    let mut scenario = GameScenario::new();
    scenario.add_enchantment_from_oracle(P0, "Ripples of Undeath", RIPPLES_OF_UNDEATH);
    scenario.add_basic_land(P0, ManaColor::Black);
    scenario.add_basic_land(P0, ManaColor::Black);
    scenario.with_library_top(P0, &["Drawn", "Milled A", "Milled B", "Milled C", "Kept"]);
    let mut runner = scenario.build();
    let milled = runner.state().players[0]
        .library
        .iter()
        .skip(1)
        .take(3)
        .copied()
        .collect();
    runner.state_mut().turn_number = 2;
    runner.state_mut().phase = Phase::Untap;
    runner.state_mut().active_player = P0;
    runner.state_mut().priority_player = P0;
    runner.state_mut().waiting_for = WaitingFor::Priority { player: P0 };
    (runner, milled)
}

/// Resolves the trigger, answering its optional payment with `pay` and its card choice
/// with the first card offered.
fn resolve_ripples(runner: &mut GameRunner, pay: bool) {
    runner.auto_advance_to_main_phase();
    for _ in 0..40 {
        match runner.state().waiting_for.clone() {
            WaitingFor::OptionalEffectChoice { .. } => {
                runner
                    .act(GameAction::DecideOptionalEffect { accept: pay })
                    .expect("optional payment");
            }
            WaitingFor::EffectZoneChoice { cards, .. }
            | WaitingFor::ChooseFromZoneChoice { cards, .. } => {
                runner
                    .act(GameAction::SelectCards {
                        cards: vec![cards[0]],
                    })
                    .expect("card choice");
            }
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => return,
            WaitingFor::Priority { .. } => {
                runner.act(GameAction::PassPriority).expect("pass");
            }
            other => panic!("unexpected prompt: {other:?}"),
        }
    }
    panic!("the trigger never finished resolving");
}

fn count_in(runner: &GameRunner, cards: &[ObjectId], zone: Zone) -> usize {
    cards
        .iter()
        .filter(|id| runner.state().objects[id].zone == zone)
        .count()
}

#[test]
fn paying_puts_one_milled_card_into_hand() {
    let (mut runner, milled) = ripples_game();
    resolve_ripples(&mut runner, true);
    assert_eq!(runner.life(P0), 17, "the payment's 3 life");
    assert_eq!(
        count_in(&runner, &milled, Zone::Hand),
        1,
        "one milled card put into the hand"
    );
    assert_eq!(
        count_in(&runner, &milled, Zone::Graveyard),
        2,
        "the other two stay in the graveyard"
    );
}

#[test]
fn declining_keeps_the_milled_cards_and_life() {
    let (mut runner, milled) = ripples_game();
    resolve_ripples(&mut runner, false);
    assert_eq!(runner.life(P0), 20);
    assert_eq!(count_in(&runner, &milled, Zone::Graveyard), 3);
}
