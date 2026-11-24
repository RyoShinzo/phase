//! The fork's own tests of this module (RyoShinzo/phase, mtg-fixes), beside upstream's in `meld_tests.rs`.
//! They live in a file of our own so that a merge of an upstream release never meets them; the
//! helpers they share with upstream's tests are copied from there.

use crate::database::mtgjson::{AtomicCard, AtomicIdentifiers};
use crate::types::card::CardFace;
use crate::types::keywords::Keyword;

fn parse_face(card: &AtomicCard) -> CardFace {
    crate::database::synthesis::build_oracle_face(card, None)
}

/// Build an `AtomicCard` for a single card face from its oracle `text`.
fn atomic(name: &str, type_line: &str, types: &[&str], text: &str) -> AtomicCard {
    AtomicCard {
        name: name.to_string(),
        mana_cost: Some("{4}{W}{W}".to_string()),
        colors: vec!["W".to_string()],
        color_identity: vec!["W".to_string()],
        power: Some("4".to_string()),
        toughness: Some("3".to_string()),
        loyalty: None,
        defense: None,
        text: Some(text.to_string()),
        layout: "meld".to_string(),
        type_line: Some(type_line.to_string()),
        types: types.iter().map(|s| s.to_string()).collect(),
        subtypes: Vec::new(),
        supertypes: vec!["Legendary".to_string()],
        keywords: None,
        side: Some("a".to_string()),
        face_name: None,
        mana_value: 6.0,
        legalities: Default::default(),
        leadership_skills: None,
        printings: Vec::new(),
        rulings: Vec::new(),
        is_game_changer: false,
        identifiers: AtomicIdentifiers {
            scryfall_oracle_id: Some(format!("{}-oracle", name.to_lowercase())),
            scryfall_id: Some(format!("{}-face", name.to_lowercase())),
        },
        foreign_data: Vec::new(),
        related_cards: crate::database::mtgjson::SetRelatedCards::default(),
    }
}

/// CR 702 + CR 113.3: a keyword the card only offers as a choice is not its
/// own. Angelic Skirmisher's data lists vigilance beside flying; only flying is
/// printed as a keyword line.
#[test]
fn keyword_offered_as_a_choice_is_not_a_printed_keyword() {
    let text = "Flying\nAt the beginning of each combat, choose first strike, vigilance, or lifelink. Creatures you control gain that ability until end of turn.";
    let mut card = atomic(
        "Angelic Skirmisher",
        "Creature — Angel",
        &["Creature"],
        text,
    );
    card.subtypes = vec!["Angel".to_string()];
    card.keywords = Some(vec!["Flying".to_string(), "Vigilance".to_string()]);
    let face = parse_face(&card);
    // allow-raw-authority: this asserts the synthesized CardFace's printed keywords, not a live object's effective keywords
    assert!(face.keywords.contains(&Keyword::Flying));
    // allow-raw-authority: this asserts the synthesized CardFace's printed keywords, not a live object's effective keywords
    assert!(!face.keywords.contains(&Keyword::Vigilance));
}
