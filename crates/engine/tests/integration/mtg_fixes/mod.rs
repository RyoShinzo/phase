//! The fork's own engine integration tests (RyoShinzo/phase, branch mtg-fixes). They live in this
//! module, registered by one line at the end of `main.rs`, so that a merge of an upstream release never
//! meets them in upstream's shard roots or test files. A module here may use `crate::rules` and
//! `crate::support`, since `main.rs` declares both.

mod akromas_will_commander_both_modes;
mod alternative_cost_without_mana_takes_cost_changes;
mod bridgeworks_battle_up_to_one_fight;
mod costed_mana_ability_generic_reservation;
mod entered_under_your_control_last_turn;
mod counter_batch_paused_per_recipient;
mod court_of_ambition;
mod curse_static_effects;
mod flawless_maneuver_free_with_commander;
mod giada_angel_counters;
mod holy_frazzle_cannon_shared_type_counters;
mod issue_654_stridehangar_automaton;
mod kindred_discovery_each_attacker;
mod mixed_alternative_cost_replaces_mana_cost;
mod ninja_teen_sneak_from_graveyard;
mod notorious_throng_damage_and_prowl;
mod other_than_trigger_object_resolution;
mod resplendent_marshal_another_excludes_dying_source;
mod returned_land_taps_for_mana_first;
mod ripples_of_undeath_paid_choice;
mod tenuous_truce_enchanted_opponent;
mod titan_of_littjara_shares_type_with_itself;
mod token_created_attacking_chooses_defender;
mod vanquishers_banner_only_your_spells;
