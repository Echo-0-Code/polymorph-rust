//! What the game's own code calls (the mixins in `src/mixins/`, compiled into the game). Plain API types only:
//! the game converts its stacks before calling.
//!
//! The client and the integrated server run in one process, so the choice made on the screen is the one the
//! server's crafting uses.

use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};

use mod_api::ItemStack;

use crate::choice::Choices;

static CHOICES: Mutex<Option<Choices<ItemStack>>> = Mutex::new(None);
static NAMES: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);

pub(crate) fn with<R>(f: impl FnOnce(&mut Choices<ItemStack>) -> R) -> R {
    let mut guard: MutexGuard<'_, _> = CHOICES.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Choices::default))
}

/// Same kind of item, count aside (`ItemStack.isSameItemSameComponents`).
pub(crate) fn same(a: &ItemStack, b: &ItemStack) -> bool {
    a.is_stackable_with(b)
}

/// The matcher found more than one recipe for `grid` (`size` by `size`, row by row): which one it makes,
/// an index into `ids`.
pub fn choose(size: usize, grid: Vec<Option<ItemStack>>, ids: Vec<String>, outputs: Vec<ItemStack>) -> usize {
    let n = ids.len();
    let ids = ids.iter().map(|id| qualified(id)).collect();
    let chosen = with(|c| c.choose(size, grid, ids, outputs, same));
    chosen.min(n.saturating_sub(1))
}

/// A recipe id that lives as long as the game: the matcher returns ids borrowed from the recipe list, and a
/// mixin helper can't name that borrow. Each id is kept once, so this holds at most one copy of each recipe's id.
pub fn intern(id: &str) -> &'static str {
    let mut guard = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let names = guard.get_or_insert_with(HashSet::new);
    if let Some(&kept) = names.get(id) {
        return kept;
    }
    let kept: &'static str = Box::leak(id.to_owned().into_boxed_str());
    names.insert(kept);
    kept
}

/// A viewer's "+" (JEI) moves recipe `id` into the open grid.
pub fn prefer(id: &str) {
    let id = qualified(id);
    with(|c| c.prefer(&id));
}

/// The server's tick: whether a choice changed, so the open grids' results must be made again.
pub fn take_changed() -> bool {
    with(|c| c.take_changed())
}

/// `minecraft:` added to a bare id, as the game names vanilla's recipes by file name alone.
pub fn qualified(id: &str) -> String {
    if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
}
