//! Our tests beside upstream's file of the same name, in a file of our own so that a merge of
//! upstream never meets them. The helpers they share with upstream's file are copied from it.

use engine::game::scenario::{GameScenario, P0, P1};
use engine::types::counter::CounterType;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const GIADA: &str = "Flying, vigilance\nEach other Angel you control enters with \
an additional +1/+1 counter on it for each Angel you already control.\n{T}: Add \
{W}. Spend this mana only to cast an Angel spell.";

/// The +1/+1 counters on each token Angel of `runner`'s board, sorted.
fn token_angel_counters(runner: &engine::game::scenario::GameRunner) -> Vec<u32> {
    let mut counters: Vec<u32> = runner
        .state()
        .objects
        .values()
        .filter(|o| {
            o.is_token
                && o.zone == Zone::Battlefield
                && o.card_types.subtypes.iter().any(|s| s == "Angel")
        })
        .map(|o| {
            o.counters
                .get(&CounterType::Plus1Plus1)
                .copied()
                .unwrap_or(0)
        })
        .collect();
    counters.sort();
    counters
}

/// CR 614.12: the tokens of one instruction enter at the same time, so each is
/// modified by Giada counting only the Angels there before the event. Two Angel
/// tokens with Giada the only Angel: one counter each, not 1 and 2.
#[test]
fn giada_two_tokens_of_one_instruction_count_only_angels_before_them() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario
        .add_creature_from_oracle(P0, "Giada, Font of Hope", 2, 2, GIADA)
        .with_subtypes(vec!["Angel"]);
    let token_maker = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Twin Summons",
            false,
            "Create two 4/4 white Angel creature tokens with flying.",
        )
        .id();
    let mut runner = scenario.build();
    runner.cast(token_maker).resolve();
    assert_eq!(token_angel_counters(&runner), vec![1, 1]);
}

/// CR 702.116a: myriad's copies are created by one instruction and enter at the
/// same time (Herald of the Host ruling 2015-11-04, Battle Angels of Tyr ruling
/// 2022-06-10), so with Giada each copy counts Giada and the attacking Angel,
/// never the other copy: two counters on each, where creating them one after
/// another gave the second copy three.
#[test]
fn giada_myriad_copies_enter_at_the_same_time() {
    use engine::game::combat::AttackTarget;
    use engine::types::actions::GameAction;
    use engine::types::game_state::WaitingFor;

    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::BeginCombat);
    scenario
        .add_creature_from_oracle(P0, "Giada, Font of Hope", 2, 2, GIADA)
        .with_subtypes(vec!["Angel"]);
    let herald = scenario
        .add_creature_from_oracle(P0, "Myriad Angel", 4, 4, "Flying\nMyriad")
        .with_subtypes(vec!["Angel"])
        .id();
    let mut runner = scenario.build();
    runner
        .declare_attackers(&[(herald, AttackTarget::Player(P1))])
        .expect("declare the attack");
    for _ in 0..20 {
        match runner.state().waiting_for.clone() {
            WaitingFor::OptionalEffectChoice { .. } => {
                runner
                    .act(GameAction::DecideOptionalEffect { accept: true })
                    .expect("accept myriad");
            }
            WaitingFor::OrderTriggers { triggers, .. } => {
                let order: Vec<usize> = (0..triggers.len()).collect();
                runner
                    .act(GameAction::OrderTriggers { order })
                    .expect("order");
            }
            WaitingFor::Priority { .. } if !runner.state().stack.is_empty() => {
                runner.act(GameAction::PassPriority).expect("pass");
            }
            _ => break,
        }
    }
    assert_eq!(
        token_angel_counters(&runner),
        vec![2, 2],
        "each copy counts Giada and the Myriad Angel, not the other copy"
    );
    let defenders: std::collections::BTreeSet<u8> = runner
        .state()
        .combat
        .as_ref()
        .expect("combat")
        .attackers
        .iter()
        .filter(|a| runner.state().objects[&a.object_id].is_token)
        .map(|a| a.defending_player.0)
        .collect();
    assert_eq!(
        defenders,
        [2, 3].into_iter().collect(),
        "one copy attacks each other opponent"
    );
}

/// CR 614.12: the copies of one copy-token instruction enter at the same time, so
/// with Giada each copy of the Angel counts Giada and the Angel, never the other
/// copy: two counters on each.
#[test]
fn giada_copies_of_one_instruction_count_only_angels_before_them() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario
        .add_creature_from_oracle(P0, "Giada, Font of Hope", 2, 2, GIADA)
        .with_subtypes(vec!["Angel"]);
    let angel = scenario
        .add_creature_from_oracle(P0, "Serra Angel", 4, 4, "Flying, vigilance")
        .with_subtypes(vec!["Angel"])
        .id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Twin Copies",
            false,
            "Create two tokens that are copies of target creature you control.",
        )
        .id();
    let mut runner = scenario.build();
    runner.cast(spell).target_objects(&[angel]).resolve();
    assert_eq!(token_angel_counters(&runner), vec![2, 2]);
}
