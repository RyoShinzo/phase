//! CR 614.1c + CR 611.2a: the "and with …" / "and as a …" entry grants a
//! replacement folds onto a battlefield entry: a permanent's zone change
//! (`ProposedEvent::ZoneChange::enter_with_grants`) or a token batch
//! (`TokenSpec::enter_with_grants`). Lives in phase-ast so the token
//! specification can carry them like its entry counters.

use serde::{Deserialize, Deserializer, Serialize};

use super::ability::AbilityDefinition;
use super::player::PlayerId;

/// CR 614.1c + CR 611.2a: one "and with …" entry grant riding a battlefield
/// entry (`ZoneChange::enter_with_grants`). The grant is a single stripped
/// chain link — a `GenericEffect` carrying only `static_abilities` with
/// `Duration::UntilHostLeavesPlay` for `SelfRef`, no `sub_ability` or
/// `else_ability` — plus the granting replacement's controller (CR 109.5),
/// so delivery installs exactly what the post-entry drain would have, for
/// every applicable same-entry replacement instead of just the first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnterWithGrant {
    pub grant: AbilityDefinition,
    pub controller: PlayerId,
}

/// CR 614.1c + CR 611.2a + CR 616.1: the entry grants riding one battlefield entry
/// (`ZoneChange::enter_with_grants`), in application order. Never empty: an entry with no
/// grant carries `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct EnterWithGrants(Vec<EnterWithGrant>);

impl EnterWithGrants {
    /// The single authority that adds grants to an entry: appends `grants` in order, creating
    /// the box on the first one, so `Some` is never empty.
    pub fn append(
        slot: &mut Option<Box<EnterWithGrants>>,
        grants: impl IntoIterator<Item = EnterWithGrant>,
    ) {
        let mut grants = grants.into_iter().peekable();
        if grants.peek().is_some() {
            slot.get_or_insert_with(|| Box::new(Self(Vec::new())))
                .0
                .extend(grants);
        }
    }

    /// The carried grants of an entry, in application order (`[]` when it carries none).
    pub fn of(grants: &Option<Box<EnterWithGrants>>) -> &[EnterWithGrant] {
        grants.as_deref().map_or(&[], |grants| grants.0.as_slice())
    }

    /// Wire reader for `ZoneChange::enter_with_grants`: an empty list reads as `None`, the same
    /// as an absent key, so a decoded `Some` is never empty either.
    pub fn deserialize_nonempty<'de, D>(deserializer: D) -> Result<Option<Box<Self>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<Vec<EnterWithGrant>>::deserialize(deserializer)?
            .filter(|grants| !grants.is_empty())
            .map(|grants| Box::new(Self(grants))))
    }
}
