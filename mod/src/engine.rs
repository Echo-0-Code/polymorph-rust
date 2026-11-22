//! Polymorph's state in this game, and the message a client's pick travels in.
//!
//! The hook `crafting_recipe_choice` asks [`choose`] on both sides: the client's answer is its prediction,
//! the server's is what is made. A pick on the client screen is kept here at once (so the prediction follows)
//! and sent to the server as [`Pick`], whose handler keeps it there too. Choices are kept per set of matching
//! recipes, the same on both sides, so both answer alike.

use std::sync::{Mutex, MutexGuard};

use mod_api::ItemStack;

use crate::choice::Choices;

/// The client's pick, sent to the server (`play_to_server`).
pub const PICK: &str = "polymorph:pick";

static CHOICES: Mutex<Option<Choices<ItemStack>>> = Mutex::new(None);

pub(crate) fn with<R>(f: impl FnOnce(&mut Choices<ItemStack>) -> R) -> R {
    let mut guard: MutexGuard<'_, _> = CHOICES.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Choices::default))
}

/// Same kind of item, count aside (`ItemStack.isSameItemSameComponents`).
pub(crate) fn same(a: &ItemStack, b: &ItemStack) -> bool {
    a.is_stackable_with(b)
}

/// The matcher found more than one recipe for `grid` (`width` wide, row by row): which one it makes, an
/// index into `ids`.
pub fn choose(width: usize, grid: &[Option<ItemStack>], ids: &[String], outputs: &[ItemStack]) -> usize {
    let ids: Vec<String> = ids.iter().map(|id| qualified(id)).collect();
    let n = ids.len();
    let chosen = with(|c| c.choose(width, grid.to_vec(), ids, outputs.to_vec(), same));
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
