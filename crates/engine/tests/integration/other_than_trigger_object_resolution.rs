//! `FilterProp::OtherThanTriggerObject` excludes the object that caused the
//! trigger (CR 603.4 + CR 109.3: "other" in a trigger names every object but
//! the triggering one), while the triggered ability resolves as well as when it
//! fires, and by identity (CR 400.7: a permanent that left and returned is a new
//! object, so it is "other").
//!
//! Before the fix the filter's per-object arm read only a binding that a
//! zone-change event filter sets, so in an effect a resolving trigger applied it
//! matched the triggering object itself (an attacker got two counters, issue
//! #40), and the count and aggregate resolvers dropped the trigger object by
//! storage id, so a permanent blinked before the trigger resolved was not "other"
//! (Valakut, the Molten Pinnacle; Selvala, Heart of the Wilds).

use engine::game::combat::AttackTarget;
use engine::game::effects::attach;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::trigger_index::reindex_object_triggers;
use engine::game::triggers::{drain_order_triggers_with_identity, process_triggers};
use engine::game::zone_pipeline::{move_object_for_test, ZoneMoveRequest};
use engine::game::zones::create_object;
use engine::types::ability::{AbilityDefinition, Effect, FilterProp, TargetFilter, TargetRef};
use engine::types::actions::GameAction;
use engine::types::card_type::CoreType;
use engine::types::counter::CounterType;
use engine::types::events::GameEvent;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::{CardId, ObjectId};
use engine::types::mana::ManaColor;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

fn plus_counters(runner: &GameRunner, id: ObjectId) -> u32 {
    runner.state().objects[&id]
        .counters
        .get(&CounterType::Plus1Plus1)
        .copied()
        .unwrap_or(0)
}

/// Rewrite `from` to `to` in the property list of every `PutCounterAll` filter
/// of `id`'s triggers, and return how many properties were rewritten. Builds
/// the effect filter the parser does not emit yet, so the engine arm is tested
/// on the very resolution path a parsed card takes.
fn rewrite_put_counter_all_prop(
    runner: &mut GameRunner,
    id: ObjectId,
    from: &FilterProp,
    to: FilterProp,
) -> usize {
    fn walk(ability: &mut AbilityDefinition, from: &FilterProp, to: &FilterProp) -> usize {
        let mut rewritten = 0;
        if let Effect::PutCounterAll {
            target: TargetFilter::Typed(typed),
            ..
        } = ability.effect.as_mut()
        {
            for prop in &mut typed.properties {
                if prop == from {
                    *prop = to.clone();
                    rewritten += 1;
                }
            }
        }
        if let Some(sub) = ability.sub_ability.as_deref_mut() {
            rewritten += walk(sub, from, to);
        }
        rewritten
    }
    let state = runner.state_mut();
    let obj = state.objects.get_mut(&id).expect("source exists");
    let mut rewritten = 0;
    for trigger in std::sync::Arc::make_mut(&mut obj.base_trigger_definitions) {
        if let Some(execute) = trigger.execute.as_deref_mut() {
            rewritten += walk(execute, from, &to);
        }
    }
    obj.materialize_base_trigger_definitions();
    reindex_object_triggers(state, id);
    rewritten
}

/// Holy Frazzle-Cannon (Scryfall): "Whenever equipped creature attacks, put a
/// +1/+1 counter on that creature and each other creature you control that shares
/// a creature type with it." The parsed fan-out excludes the attacker with
/// `DistinctFrom { TriggeringSource }`; here that exclusion is
/// `OtherThanTriggerObject`, which must leave the attacker out just the same.
const HOLY_FRAZZLE_CANNON: &str = "Whenever equipped creature attacks, put a +1/+1 counter on that creature and each other creature you control that shares a creature type with it.\n\
    Equip {1}";

fn frazzle_cannon_with_other_than_trigger_object(second_attacker: bool) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let cannon = scenario
        .add_artifact_from_oracle(P0, "Holy Frazzle-Cannon", HOLY_FRAZZLE_CANNON)
        .with_subtypes(vec!["Equipment"])
        .id();
    let attacker = scenario
        .add_creature(P0, "Human Knight", 2, 2)
        .with_subtypes(vec!["Human", "Knight"])
        .id();
    let knight = scenario
        .add_creature(P0, "Spirit Knight", 2, 2)
        .with_subtypes(vec!["Spirit", "Knight"])
        .id();
    let zombie = scenario
        .add_creature(P0, "Zombie", 2, 2)
        .with_subtypes(vec!["Zombie"])
        .id();
    let mut runner = scenario.build();
    runner.state_mut().all_creature_types = ["Human", "Knight", "Spirit", "Zombie"]
        .map(String::from)
        .to_vec();
    let rewritten = rewrite_put_counter_all_prop(
        &mut runner,
        cannon,
        &FilterProp::DistinctFrom {
            reference: Box::new(TargetFilter::TriggeringSource),
        },
        FilterProp::OtherThanTriggerObject,
    );
    assert_eq!(rewritten, 1, "the fan-out's exclusion is rewritten");
    attach::attach_to(runner.state_mut(), cannon, attacker);

    runner.advance_to_combat();
    let mut attackers = vec![(attacker, AttackTarget::Player(P1))];
    if second_attacker {
        attackers.push((knight, AttackTarget::Player(P1)));
    }
    runner
        .declare_attackers(&attackers)
        .expect("declare the attackers");
    runner.advance_until_stack_empty();

    assert_eq!(
        plus_counters(&runner, attacker),
        1,
        "the attacker is the trigger object: one counter, not two"
    );
    assert_eq!(plus_counters(&runner, knight), 1, "shares Knight");
    assert_eq!(plus_counters(&runner, zombie), 0, "shares no type");
}

/// Attack trigger, one attacker.
#[test]
fn attack_trigger_effect_leaves_the_attacker_out() {
    frazzle_cannon_with_other_than_trigger_object(false);
}

/// Attack trigger, two attackers declared together: the trigger object is still
/// the equipped one alone.
#[test]
fn attack_trigger_effect_leaves_the_attacker_out_among_two_attackers() {
    frazzle_cannon_with_other_than_trigger_object(true);
}

/// Enters trigger: "Whenever a creature you control enters, put a +1/+1 counter
/// on each other creature you control", with "other" read as other than the
/// entering creature. The entrant gets none, every other creature one.
#[test]
fn enters_trigger_effect_leaves_the_entrant_out() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let source = {
        let mut builder = scenario.add_creature(P0, "Test Enchantment", 0, 0);
        builder.as_enchantment();
        builder.from_oracle_text(
            "Whenever a creature you control enters, put a +1/+1 counter on each other creature you control.",
        );
        builder.id()
    };
    let bear = scenario.add_creature(P0, "Bear", 2, 2).id();
    let mut runner = scenario.build();
    let rewritten = rewrite_put_counter_all_prop(
        &mut runner,
        source,
        &FilterProp::Another,
        FilterProp::OtherThanTriggerObject,
    );
    assert_eq!(rewritten, 1, "the fan-out's \"other\" is rewritten");

    let entrant = place_creature(&mut runner, P0, "Entrant", Zone::Hand);
    let mut events = Vec::new();
    move_through_pipeline(&mut runner, entrant, Zone::Battlefield, &mut events);
    process_triggers(runner.state_mut(), &events);
    drain_order_triggers_with_identity(runner.state_mut());
    runner.advance_until_stack_empty();

    assert_eq!(
        plus_counters(&runner, entrant),
        0,
        "the entrant is the trigger object"
    );
    assert_eq!(
        plus_counters(&runner, bear),
        1,
        "another creature you control"
    );
}

fn place_creature(runner: &mut GameRunner, owner: PlayerId, name: &str, zone: Zone) -> ObjectId {
    let state = runner.state_mut();
    let card_id = CardId(state.next_object_id);
    let id = create_object(state, card_id, owner, name.to_string(), zone);
    let obj = state.objects.get_mut(&id).unwrap();
    obj.card_types.core_types.push(CoreType::Creature);
    obj.base_card_types = obj.card_types.clone();
    obj.power = Some(2);
    obj.toughness = Some(2);
    id
}

fn place_mountain(runner: &mut GameRunner, owner: PlayerId, zone: Zone) -> ObjectId {
    let state = runner.state_mut();
    let card_id = CardId(state.next_object_id);
    let id = create_object(state, card_id, owner, "Mountain".to_string(), zone);
    let obj = state.objects.get_mut(&id).unwrap();
    obj.card_types.core_types.push(CoreType::Land);
    obj.card_types.subtypes.push("Mountain".to_string());
    obj.base_card_types = obj.card_types.clone();
    id
}

fn move_through_pipeline(
    runner: &mut GameRunner,
    object_id: ObjectId,
    destination: Zone,
    events: &mut Vec<GameEvent>,
) {
    assert!(
        !move_object_for_test(
            runner.state_mut(),
            ZoneMoveRequest::effect(object_id, destination, object_id),
            events,
        ),
        "the fixture move must not pause for a replacement choice"
    );
}

/// Run the trigger pass for `events`, aiming any trigger at the opponent and
/// accepting any "you may".
fn fire_and_settle(runner: &mut GameRunner, events: &[GameEvent]) {
    process_triggers(runner.state_mut(), events);
    drain_order_triggers_with_identity(runner.state_mut());
    choose_opponent_targets(runner);
}

fn choose_opponent_targets(runner: &mut GameRunner) {
    for _ in 0..10 {
        match &runner.state().waiting_for {
            WaitingFor::TriggerTargetSelection { .. } => {
                runner
                    .act(GameAction::ChooseTarget {
                        target: Some(TargetRef::Player(P1)),
                    })
                    .expect("the opponent is a legal target");
            }
            WaitingFor::OrderTriggers { .. } => {
                drain_order_triggers_with_identity(runner.state_mut());
            }
            _ => break,
        }
    }
}

fn resolve_all(runner: &mut GameRunner) {
    for _ in 0..40 {
        choose_opponent_targets(runner);
        match &runner.state().waiting_for {
            WaitingFor::OptionalEffectChoice { .. } => {
                runner
                    .act(GameAction::DecideOptionalEffect { accept: true })
                    .expect("accept the optional effect");
            }
            _ if runner.state().stack.is_empty() => break,
            _ => runner.resolve_top(),
        }
    }
}

/// Valakut, the Molten Pinnacle (Scryfall): "Whenever a Mountain you control
/// enters, if you control at least five other Mountains, you may have this land
/// deal 3 damage to any target."
const VALAKUT: &str = "This land enters tapped.\n\
    Whenever a Mountain you control enters, if you control at least five other Mountains, you may have this land deal 3 damage to any target.\n\
    {T}: Add {R}.";

/// Valakut with `others` Mountains already on the battlefield; then one more
/// Mountain enters. Returns the runner and the entering Mountain, with the
/// trigger (if any) on the stack and its target chosen.
fn valakut_with_entering_mountain(others: usize) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let valakut = scenario
        .add_land_from_oracle(P0, "Valakut, the Molten Pinnacle", VALAKUT)
        .id();
    for _ in 0..others {
        scenario.add_basic_land(P0, ManaColor::Red);
    }
    let mut runner = scenario.build();
    reindex_object_triggers(runner.state_mut(), valakut);
    let mountain = place_mountain(&mut runner, P0, Zone::Hand);
    let mut events = Vec::new();
    move_through_pipeline(&mut runner, mountain, Zone::Battlefield, &mut events);
    fire_and_settle(&mut runner, &events);
    (runner, mountain)
}

fn opponent_life(runner: &GameRunner) -> i32 {
    runner.state().players[P1.0 as usize].life
}

/// Enters trigger, count. Five other Mountains: 3 damage. Four: the entering
/// Mountain is not one of the five, so nothing.
#[test]
fn valakut_counts_mountains_other_than_the_entering_one() {
    let (mut runner, _) = valakut_with_entering_mountain(5);
    let before = opponent_life(&runner);
    resolve_all(&mut runner);
    assert_eq!(opponent_life(&runner), before - 3, "five other Mountains");

    let (mut runner, _) = valakut_with_entering_mountain(4);
    let before = opponent_life(&runner);
    assert!(
        runner.state().stack.is_empty(),
        "four other Mountains: no trigger"
    );
    resolve_all(&mut runner);
    assert_eq!(opponent_life(&runner), before, "four other Mountains");
}

/// Enters trigger, identity. Five other Mountains and a sixth enters: Valakut
/// triggers. Before it resolves, one of the five goes to the graveyard and the
/// entering Mountain is blinked. CR 400.7: the Mountain that returns is a new
/// object, so at the CR 603.4 recheck it is one of five other Mountains (four old
/// ones and itself) and Valakut deals its damage. Its own trigger does not fire:
/// it is not other than itself, so it sees four.
#[test]
fn valakut_counts_the_blinked_entering_mountain_as_another_mountain() {
    let (mut runner, mountain) = valakut_with_entering_mountain(5);
    assert_eq!(
        runner.state().stack.len(),
        1,
        "the first trigger is on the stack"
    );
    let gone = *runner
        .state()
        .battlefield
        .iter()
        .find(|id| {
            **id != mountain
                && runner.state().objects[*id]
                    .card_types
                    .subtypes
                    .iter()
                    .any(|subtype| subtype == "Mountain")
        })
        .expect("another Mountain");
    let mut events = Vec::new();
    move_through_pipeline(&mut runner, gone, Zone::Graveyard, &mut events);
    move_through_pipeline(&mut runner, mountain, Zone::Exile, &mut events);
    move_through_pipeline(&mut runner, mountain, Zone::Battlefield, &mut events);
    assert_eq!(runner.state().objects[&mountain].zone, Zone::Battlefield);
    fire_and_settle(&mut runner, &events);
    assert_eq!(
        runner.state().stack.len(),
        1,
        "the returned Mountain sees only four other Mountains"
    );
    let before = opponent_life(&runner);
    resolve_all(&mut runner);
    assert_eq!(
        opponent_life(&runner),
        before - 3,
        "the returned Mountain is another Mountain for the first trigger"
    );
}

/// Platoon Dispenser (Scryfall): "At the beginning of your end step, if you
/// control two or more other creatures, draw a card." A phase trigger has no
/// trigger object, so "other" is other than Platoon Dispenser itself.
const PLATOON_DISPENSER: &str =
    "At the beginning of your end step, if you control two or more other creatures, draw a card.\n\
    {3}{W}: Create a 1/1 colorless Soldier artifact creature token.\n\
    Unearth {2}{W}{W}";

fn platoon_dispenser_draws(other_creatures: usize) -> usize {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_creature_from_oracle(P0, "Platoon Dispenser", 4, 6, PLATOON_DISPENSER);
    for i in 0..other_creatures {
        scenario.add_creature(P0, &format!("Soldier {i}"), 1, 1);
    }
    for _ in 0..10 {
        scenario.add_card_to_library_top(P0, "Plains");
    }
    let mut runner = scenario.build();
    let before = runner.state().players[P0.0 as usize].hand.len();
    runner.advance_to_combat();
    runner.declare_attackers(&[]).expect("attack with nothing");
    runner.advance_to_end_step();
    assert_eq!(runner.state().phase, Phase::End, "the end step is reached");
    runner.advance_until_stack_empty();
    runner.state().players[P0.0 as usize].hand.len() - before
}

/// Phase trigger. Two other creatures: a card. One other creature: none, as
/// Platoon Dispenser is not other than itself.
#[test]
fn platoon_dispenser_counts_creatures_other_than_itself() {
    assert_eq!(platoon_dispenser_draws(2), 1, "two other creatures");
    assert_eq!(platoon_dispenser_draws(1), 0, "one other creature");
}

/// Selvala, Heart of the Wilds (Scryfall): "Whenever another creature enters, its
/// controller may draw a card if its power is greater than each other creature's
/// power." Ruling (2016-08-23): the comparison is made as the ability resolves,
/// and a creature that has left uses its power when it left.
const SELVALA: &str = "Whenever another creature enters, its controller may draw a card if its power is greater than each other creature's power.\n\
    {G}, {T}: Add X mana in any combination of colors, where X is the greatest power among creatures you control.";

/// Enters trigger, aggregate. A 5/5 enters beside Selvala (2/3) and is blinked
/// before the trigger resolves. CR 400.7: the 5/5 that returns is another
/// creature, so the first trigger compares 5 (the departed creature's power) with
/// 5 and draws nothing; the returned creature's own trigger compares 5 with 2 and
/// draws one card. One card in all.
#[test]
fn selvala_compares_with_the_blinked_creature_as_another_creature() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let selvala = scenario
        .add_creature_from_oracle(P0, "Selvala, Heart of the Wilds", 2, 3, SELVALA)
        .id();
    for _ in 0..10 {
        scenario.add_card_to_library_top(P0, "Plains");
    }
    let mut runner = scenario.build();
    reindex_object_triggers(runner.state_mut(), selvala);
    let big = place_creature(&mut runner, P0, "Big", Zone::Hand);
    {
        let obj = runner.state_mut().objects.get_mut(&big).unwrap();
        obj.power = Some(5);
        obj.toughness = Some(5);
        obj.base_power = Some(5);
        obj.base_toughness = Some(5);
    }
    let mut events = Vec::new();
    move_through_pipeline(&mut runner, big, Zone::Battlefield, &mut events);
    fire_and_settle(&mut runner, &events);
    assert_eq!(
        runner.state().stack.len(),
        1,
        "Selvala triggers for the 5/5"
    );

    let mut events = Vec::new();
    move_through_pipeline(&mut runner, big, Zone::Exile, &mut events);
    move_through_pipeline(&mut runner, big, Zone::Battlefield, &mut events);
    fire_and_settle(&mut runner, &events);
    assert_eq!(
        runner.state().stack.len(),
        2,
        "and again for the returned 5/5"
    );
    assert_eq!(runner.state().objects[&big].power, Some(5));

    let before = runner.state().players[P0.0 as usize].hand.len();
    resolve_all(&mut runner);
    assert_eq!(
        runner.state().players[P0.0 as usize].hand.len() - before,
        1,
        "only the returned creature's trigger draws"
    );
}
