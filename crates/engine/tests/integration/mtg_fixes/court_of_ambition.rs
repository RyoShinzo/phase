//! Our tests beside upstream's file of the same name, in a file of our own so that a merge of
//! upstream never meets them. The helpers they share with upstream's file are copied from it.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::ability::{AbilityCost, QuantityExpr};
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const P2: PlayerId = PlayerId(2);
const COURT_OF_AMBITION: &str = "When this enchantment enters, you become the monarch.\n\
     At the beginning of your upkeep, each opponent loses 3 life unless they discard a card. \
     If you're the monarch, instead each opponent loses 6 life unless they discard two cards.";

fn life(runner: &GameRunner, player: PlayerId) -> i32 {
    runner
        .state()
        .players
        .iter()
        .find(|p| p.id == player)
        .expect("player exists")
        .life
}

/// `player_count`-seat game with Court of Ambition already on P0's battlefield
/// and `opponent_hand` discardable cards in every opponent's hand. `monarch` seeds
/// the designation directly — a scenario-seeded permanent never fires its own
/// ETB, so the "you become the monarch" half is covered separately by
/// `court_of_ambition_etb_makes_its_controller_the_monarch`.
fn build_runner(player_count: u8, monarch: Option<PlayerId>, opponent_hand: usize) -> GameRunner {
    let mut scenario = GameScenario::new_n_player(player_count, 42);
    scenario.at_phase(Phase::Untap);
    scenario.add_enchantment_from_oracle(P0, "Court of Ambition", COURT_OF_AMBITION);

    for seat in 1..player_count {
        let pid = PlayerId(seat);
        let names: Vec<String> = (0..opponent_hand)
            .map(|i| format!("Filler Card {seat}-{i}"))
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        scenario.with_cards_in_hand(pid, &refs);
    }

    let mut runner = scenario.build();
    runner.state_mut().turn_number = 2;
    runner.state_mut().active_player = P0;
    runner.state_mut().priority_player = P0;
    runner.state_mut().monarch = monarch;
    runner
}

/// Drive to P0's upkeep and let the trigger resolve until it pauses on the
/// first opponent's unless-payment prompt (CR 118.12a).
fn fire_upkeep(runner: &mut GameRunner) {
    runner.advance_to_upkeep();
    for _ in 0..20 {
        if matches!(runner.state().waiting_for, WaitingFor::UnlessPayment { .. }) {
            return;
        }
        if runner.state().stack.is_empty() && runner.state().phase != Phase::Upkeep {
            break;
        }
        if runner.act(GameAction::PassPriority).is_err() {
            break;
        }
    }
    panic!(
        "upkeep trigger never surfaced an unless-payment prompt: {:?}",
        runner.state().waiting_for
    );
}

/// Assert the pending prompt is an unless-discard of `expected_count` cards
/// owed by `expected_payer`. CR 118.12a: the payer is the scoped opponent of
/// this fan-out iteration, never the controller.
fn expect_discard_prompt(runner: &GameRunner, expected_payer: PlayerId, expected_count: i32) {
    match &runner.state().waiting_for {
        WaitingFor::UnlessPayment { player, cost, .. } => {
            assert_eq!(
                *player, expected_payer,
                "the scoped opponent pays their own unless-cost, not the controller"
            );
            match cost {
                AbilityCost::Discard { count, filter, .. } => {
                    assert_eq!(
                        *count,
                        QuantityExpr::Fixed {
                            value: expected_count
                        },
                        "unless-cost card count"
                    );
                    assert!(
                        filter.is_none(),
                        "any card may be discarded, got {filter:?}"
                    );
                }
                other => panic!("expected a Discard unless-cost, got {other:?}"),
            }
        }
        other => panic!("expected UnlessPayment prompt, got {other:?}"),
    }
}

/// Pay the pending unless-discard by submitting one card per re-prompt round
/// trip, until the discard loop is exhausted.
fn pay_discard(runner: &mut GameRunner) {
    runner
        .act(GameAction::PayUnlessCost { pay: true })
        .expect("choosing to pay the discard cost must be accepted");
    for _ in 0..8 {
        let card = match &runner.state().waiting_for {
            WaitingFor::WardDiscardChoice { cards, .. } => {
                *cards.first().expect("an eligible card to discard")
            }
            _ => return,
        };
        runner
            .act(GameAction::SelectCards { cards: vec![card] })
            .expect("discard selection must be accepted");
    }
    panic!("discard loop did not terminate");
}

/// CR 118.12 + CR 608.2c: a paid discard suppresses only its own opponent's
/// loss. With four players, P1 and P2 pay and P3 declines: P3 still loses 3.
/// The leg after a resumed leg rides that leg's sub-chain, and the discard
/// choice handler used to drop it with the suppressed effect (Rottenmouth
/// Viper's third opponent lost nothing).
#[test]
fn court_of_ambition_third_opponent_is_polled_after_two_paid_discards() {
    let mut runner = build_runner(4, None, 3);
    let p3 = PlayerId(3);
    fire_upkeep(&mut runner);
    for payer in [P1, P2] {
        for _ in 0..10 {
            if matches!(runner.state().waiting_for, WaitingFor::UnlessPayment { .. }) {
                break;
            }
            if runner.act(GameAction::PassPriority).is_err() {
                break;
            }
        }
        expect_discard_prompt(&runner, payer, 1);
        pay_discard(&mut runner);
    }
    for _ in 0..10 {
        if matches!(runner.state().waiting_for, WaitingFor::UnlessPayment { .. }) {
            break;
        }
        if runner.act(GameAction::PassPriority).is_err() {
            break;
        }
    }
    expect_discard_prompt(&runner, p3, 1);
    runner
        .act(GameAction::PayUnlessCost { pay: false })
        .expect("declining must be accepted");
    runner.advance_until_stack_empty();

    assert_eq!(life(&runner, P1), 20, "P1 paid");
    assert_eq!(life(&runner, P2), 20, "P2 paid");
    assert_eq!(life(&runner, p3), 17, "P3 declined, so P3 loses 3 life");
}
