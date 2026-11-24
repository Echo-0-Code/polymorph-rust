//! Polymorph's state in this game, and the message a client's pick travels in.
//!
//! The hook `crafting_recipe_choice` asks [`choose`] on both sides: the client's answer is its prediction,
//! the server's is what is made. A pick on the client screen is kept here at once (so the prediction follows)
//! and sent to the server as [`Pick`], whose handler keeps it there too. As in Polymorph, each player keeps
//! their own choices (the hook names the player whose grid it is); a grid matched on nobody's behalf (a
//! crafter block, a mod's lookup) makes the first match, as vanilla.

use std::sync::{Mutex, MutexGuard};

use mod_api::ItemStack;

use crate::choice::{Choices, Players};

/// The client's pick, sent to the server (`play_to_server`).
pub const PICK: &str = "polymorph:pick";

static PLAYERS: Mutex<Option<Players<ItemStack>>> = Mutex::new(None);

fn players() -> MutexGuard<'static, Option<Players<ItemStack>>> {
    PLAYERS.lock().unwrap_or_else(|e| e.into_inner())
}

/// `f` on `player`'s choices.
pub(crate) fn with<R>(player: u128, f: impl FnOnce(&mut Choices<ItemStack>) -> R) -> R {
    f(players().get_or_insert_with(Players::default).of(player))
}

/// A player left: their choices go.
pub(crate) fn forget(player: u128) {
    if let Some(all) = players().as_mut() {
        all.forget(player);
    }
}

/// Same kind of item, count aside (`ItemStack.isSameItemSameComponents`).
pub(crate) fn same(a: &ItemStack, b: &ItemStack) -> bool {
    a.is_stackable_with(b)
}

/// The matcher found more than one recipe for `grid` (`width` wide, row by row), crafted by `player`
/// (none for a grid nobody crafts in): which one it makes, an index into `ids`.
pub fn choose(
    player: Option<u128>,
    width: usize,
    grid: &[Option<ItemStack>],
    ids: &[String],
    outputs: &[ItemStack],
) -> usize {
    let ids: Vec<String> = ids.iter().map(|id| qualified(id)).collect();
    let n = ids.len();
    let chosen = players().get_or_insert_with(Players::default).choose(
        player,
        width,
        grid.to_vec(),
        ids,
        outputs.to_vec(),
        same,
    );
    chosen.min(n.saturating_sub(1))
}

/// `minecraft:` added to a bare id.
pub fn qualified(id: &str) -> String {
    if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
}

/// A pick: recipe `id` among the matching recipes `set` (Polymorph's `CPacketPlayerRecipeSelection`, which
/// sends the recipe id; the set is sent too because the server keeps a choice per set).
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    pub id: String,
    pub set: Vec<String>,
}

impl Pick {
    /// One id per line, the picked one first.
    pub fn encode(&self) -> Vec<u8> {
        std::iter::once(&self.id).chain(&self.set).cloned().collect::<Vec<_>>().join("\n").into_bytes()
    }

    pub fn decode(bytes: &[u8]) -> Option<Pick> {
        let text = std::str::from_utf8(bytes).ok()?;
        let mut lines = text.split('\n').map(str::to_owned);
        let id = lines.next().filter(|id| !id.is_empty())?;
        let set: Vec<String> = lines.filter(|l| !l.is_empty()).collect();
        (set.len() >= 2 && set.len() <= 64 && set.contains(&id)).then_some(Pick { id, set })
    }
}
