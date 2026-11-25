//! "Untap N tapped creatures you control" as a cost, Halo Fountain's three
//! abilities.
//!
//! The cost parsed as an effect-as-cost (`SetTapState Untap` of one object),
//! which the payment authority refused, so none of the abilities could be
//! activated; "untap two" and "untap fifteen" also lost their count. It is now
//! the untap direction of `AbilityCost::TapCreatures`: the payer chooses that
//! many tapped creatures they control and untaps them.
//!
//! CR 701.26b: only tapped permanents can be untapped, so an untapped creature
//! can't pay, and the cost can't be paid without that many tapped creatures.

use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::types::ability::TapStateChange;
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, PayCostKind, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaColor;
use engine::types::phase::Phase;

const HALO_FOUNTAIN_ORACLE: &str = "{W}, {T}, Untap a tapped creature you control: Create a 1/1 green and white Citizen creature token.\n{W}{W}, {T}, Untap two tapped creatures you control: Draw a card.\n{W}{W}{W}{W}{W}, {T}, Untap fifteen tapped creatures you control: You win the game.";

const CITIZEN: usize = 0;
const DRAW: usize = 1;
const WIN: usize = 2;

struct Board {
    runner: GameRunner,
    fountain: ObjectId,
    tapped: Vec<ObjectId>,
    untapped: ObjectId,
}

/// Halo Fountain, five Plains, `tapped` tapped Grizzly Bears and one untapped.
fn fountain_board(tapped: usize) -> Board {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_library_top(P0, &["Island", "Island"]);
    for _ in 0..5 {
        scenario.add_basic_land(P0, ManaColor::White);
    }
    let fountain = scenario
        .add_artifact_from_oracle(P0, "Halo Fountain", HALO_FOUNTAIN_ORACLE)
        .id();
    let tapped_ids: Vec<ObjectId> = (0..tapped)
        .map(|_| scenario.add_creature(P0, "Grizzly Bears", 2, 2).id())
        .collect();
    let untapped = scenario.add_creature(P0, "Grizzly Bears", 2, 2).id();
    let mut runner = scenario.build();
    for id in &tapped_ids {
        runner.state_mut().objects.get_mut(id).unwrap().tapped = true;
    }
    Board {
        runner,
        fountain,
        tapped: tapped_ids,
        untapped,
    }
}

fn activate(board: &mut Board, index: usize) -> Result<(), String> {
    board
        .runner
        .act(GameAction::ActivateAbility {
            source_id: board.fountain,
            ability_index: index,
            payment_mode: CastPaymentMode::Auto,
        })
        .map(|_| ())
        .map_err(|err| format!("{err:?}"))
}

/// The untap prompt the activation stopped at: its choices and count.
fn untap_prompt(runner: &GameRunner) -> (Vec<ObjectId>, usize) {
    match &runner.state().waiting_for {
        WaitingFor::PayCost {
            kind:
                PayCostKind::TapCreatures {
                    direction: TapStateChange::Untap,
                    ..
                },
            choices,
            count,
            min_count,
            ..
        } => {
            assert_eq!(min_count, count, "a fixed count is paid whole");
            (choices.clone(), *count)
        }
        other => panic!("expected the untap-creatures prompt, got {other:?}"),
    }
}

fn citizens(runner: &GameRunner) -> usize {
    runner
        .state()
        .battlefield
        .iter()
        .filter(|id| runner.state().objects[id].name == "Citizen")
        .count()
}

/// CR 701.26b + CR 602.2b: the Citizen ability offers only the tapped
/// creatures, untaps the one chosen, and makes a Citizen when it resolves.
#[test]
fn citizen_ability_untaps_the_chosen_tapped_creature() {
    let mut board = fountain_board(2);
    activate(&mut board, CITIZEN).expect("two tapped creatures pay the Citizen ability");
    let (choices, count) = untap_prompt(&board.runner);
    assert_eq!(count, 1);
    assert!(
        board.tapped.iter().all(|id| choices.contains(id)),
        "every tapped creature is offered: {choices:?}"
    );
    assert!(
        !choices.contains(&board.untapped),
        "CR 701.26b: an untapped creature can't be untapped to pay"
    );
    let chosen = board.tapped[0];
    board
        .runner
        .act(GameAction::SelectCards {
            cards: vec![chosen],
        })
        .expect("the chosen tapped creature pays");
    let state = board.runner.state();
    assert!(!state.objects[&chosen].tapped, "the chosen creature untapped");
    assert!(state.objects[&board.tapped[1]].tapped, "the other stays tapped");
    assert!(state.objects[&board.fountain].tapped, "{{T}} tapped the Fountain");
    board.runner.advance_until_stack_empty();
    assert_eq!(citizens(&board.runner), 1, "the ability made one Citizen");
}

/// CR 701.26b: the untapped creature isn't a legal answer to the prompt.
#[test]
fn an_untapped_creature_is_refused_as_payment() {
    let mut board = fountain_board(2);
    activate(&mut board, CITIZEN).expect("reach guard: the prompt opens");
    untap_prompt(&board.runner);
    let untapped = board.untapped;
    assert!(
        board
            .runner
            .act(GameAction::SelectCards {
                cards: vec![untapped],
            })
            .is_err(),
        "an untapped creature can't pay an untap cost"
    );
}

/// CR 601.2h + CR 701.26b: the draw ability untaps exactly two tapped
/// creatures and draws one card.
#[test]
fn draw_ability_untaps_two_and_draws() {
    let mut board = fountain_board(3);
    let hand = board.runner.state().players[0].hand.len();
    activate(&mut board, DRAW).expect("three tapped creatures pay the draw ability");
    let (_, count) = untap_prompt(&board.runner);
    assert_eq!(count, 2, "\"untap two\" keeps its count");
    let chosen = vec![board.tapped[0], board.tapped[2]];
    board
        .runner
        .act(GameAction::SelectCards {
            cards: chosen.clone(),
        })
        .expect("two tapped creatures pay");
    for id in &chosen {
        assert!(!board.runner.state().objects[id].tapped);
    }
    assert!(board.runner.state().objects[&board.tapped[1]].tapped);
    board.runner.advance_until_stack_empty();
    assert_eq!(board.runner.state().players[0].hand.len(), hand + 1);
}

/// CR 118.3 + CR 701.26b: with one tapped creature the draw ability can't be
/// activated; the untapped creature doesn't count.
#[test]
fn draw_ability_needs_two_tapped_creatures() {
    let mut board = fountain_board(1);
    assert!(
        activate(&mut board, DRAW).is_err(),
        "one tapped and one untapped creature can't pay \"untap two\""
    );
    let mut reach = fountain_board(2);
    activate(&mut reach, DRAW).expect("reach guard: two tapped creatures pay");
}

/// CR 104.2b + CR 701.26b: fifteen tapped creatures pay the third ability,
/// and its controller wins; fourteen can't pay it.
#[test]
fn win_ability_takes_fifteen_tapped_creatures() {
    let mut short = fountain_board(14);
    assert!(
        activate(&mut short, WIN).is_err(),
        "fourteen tapped creatures can't pay \"untap fifteen\""
    );
    let mut board = fountain_board(15);
    activate(&mut board, WIN).expect("fifteen tapped creatures pay the win ability");
    let (choices, count) = untap_prompt(&board.runner);
    assert_eq!(count, 15);
    let chosen: Vec<ObjectId> = choices.into_iter().take(15).collect();
    board
        .runner
        .act(GameAction::SelectCards { cards: chosen })
        .expect("fifteen tapped creatures pay");
    board.runner.advance_until_stack_empty();
    assert!(
        matches!(
            board.runner.state().waiting_for,
            WaitingFor::GameOver {
                winner: Some(P0),
                ..
            }
        ),
        "the Fountain's controller wins, got {:?}",
        board.runner.state().waiting_for
    );
}
