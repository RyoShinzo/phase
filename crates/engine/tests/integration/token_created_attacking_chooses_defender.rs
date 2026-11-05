//! CR 508.4: a token created attacking, with no destination named by the effect,
//! attacks the defending player its controller chooses, not automatically the
//! player its source attacks (U.S.S. Stargazer, Here and There's Illusion).

use engine::game::combat::AttackTarget;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const ONE_TOKEN: &str =
    "Whenever this creature attacks, create a tapped and attacking 1/1 white Soldier creature token.";
const TWO_TOKENS: &str =
    "Whenever this creature attacks, create two tapped and attacking 1/1 white Soldier creature tokens.";

/// Passes priority until the next entry attack choice, ordering any triggers as listed.
fn advance_to_entry_attack_choice(runner: &mut GameRunner) -> (ObjectId, Vec<AttackTarget>) {
    for _ in 0..40 {
        match runner.state().waiting_for.clone() {
            WaitingFor::EntryAttackTargetChoice {
                object_id,
                valid_targets,
                ..
            } => return (object_id, valid_targets),
            WaitingFor::OrderTriggers { triggers, .. } => runner
                .act(GameAction::OrderTriggers {
                    order: (0..triggers.len()).collect(),
                })
                .expect("order triggers"),
            WaitingFor::Priority { .. } => runner
                .act(GameAction::PassPriority)
                .expect("pass priority toward the token's creation"),
            other => panic!("unexpected state before the entry attack choice: {other:?}"),
        };
    }
    panic!("the token's entry attack choice never came");
}

fn attacking_after_token_trigger(oracle: &str) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(3, 42);
    scenario.at_phase(Phase::PreCombatMain);
    let attacker = scenario
        .add_creature_from_oracle(P0, "Token Maker", 2, 2, oracle)
        .id();
    let mut runner = scenario.build();
    runner.advance_to_combat();
    runner
        .declare_attackers(&[(attacker, AttackTarget::Player(P1))])
        .expect("declare the token maker");
    (runner, attacker)
}

fn defender_of(runner: &GameRunner, object_id: ObjectId) -> Option<PlayerId> {
    runner
        .state()
        .combat
        .as_ref()
        .and_then(|combat| combat.attackers.iter().find(|a| a.object_id == object_id))
        .map(|a| a.defending_player)
}

/// CR 508.4: the token's controller is asked, and the token attacks the chosen
/// player while its source keeps attacking the declared one.
#[test]
fn token_created_attacking_attacks_the_chosen_defender() {
    let p2 = PlayerId(2);
    let (mut runner, attacker) = attacking_after_token_trigger(ONE_TOKEN);
    let (token, valid_targets) = advance_to_entry_attack_choice(&mut runner);
    assert!(valid_targets.contains(&AttackTarget::Player(P1)));
    assert!(valid_targets.contains(&AttackTarget::Player(p2)));
    runner
        .act(GameAction::ChooseEntryAttackTarget {
            target: AttackTarget::Player(p2),
        })
        .expect("choose the other opponent for the token");
    runner.advance_until_stack_empty();

    let token_object = runner.state().objects.get(&token).expect("token object");
    assert!(token_object.tapped, "the token enters tapped");
    assert_eq!(defender_of(&runner, token), Some(p2));
    assert_eq!(defender_of(&runner, attacker), Some(P1));
}

/// CR 508.4 + CR 608.2c: each token of a batch is asked separately, and the batch
/// resumes after each answer, so both tokens exist and attack where chosen.
#[test]
fn each_token_of_a_batch_created_attacking_chooses_its_defender() {
    let p2 = PlayerId(2);
    let (mut runner, _) = attacking_after_token_trigger(TWO_TOKENS);
    let (first, _) = advance_to_entry_attack_choice(&mut runner);
    runner
        .act(GameAction::ChooseEntryAttackTarget {
            target: AttackTarget::Player(p2),
        })
        .expect("choose a defender for the first token");
    let (second, _) = advance_to_entry_attack_choice(&mut runner);
    assert_ne!(first, second, "the second token is a new object");
    runner
        .act(GameAction::ChooseEntryAttackTarget {
            target: AttackTarget::Player(P1),
        })
        .expect("choose a defender for the second token");
    runner.advance_until_stack_empty();

    assert_eq!(defender_of(&runner, first), Some(p2));
    assert_eq!(defender_of(&runner, second), Some(P1));
}
