//! Polymorph's recipe choice, without the game: which of the recipes that match one crafting grid is made.
//!
//! Polymorph (`AbstractRecipeData.getRecipe`) keeps one selected recipe per player and makes it while it is
//! among the grid's matches, else the first match. Here the choice is kept per set of matching recipes, so a
//! conflict chosen once stays chosen when the same ingredients come back ("as long as the ingredients don't
//! change" in Polymorph's words, and also after they did). The last recipe picked by hand, and a recipe a
//! viewer's "+" asked for, are tried next, then the first match, as in Polymorph.
//!
//! The game reaches this through [`crate::engine`]; everything here is plain data so it can be tested.

use std::collections::HashMap;

/// How many grids with a conflict are remembered for the selector (the open screen asks for its own grid).
pub const RECENT: usize = 16;
/// Polymorph shows at most 15 results (`AbstractRecipeData.getRecipe`, `recipesList.size() < 15`).
pub const MAX_SHOWN: usize = 15;

/// One grid whose ingredients match more than one recipe.
#[derive(Clone, Debug, PartialEq)]
pub struct Conflict<S> {
    /// The grid's side (2 for the inventory, 3 for a table) and its cells row by row.
    pub size: usize,
    pub grid: Vec<Option<S>>,
    /// The matching recipes in the game's order, with what each makes.
    pub ids: Vec<String>,
    pub outputs: Vec<S>,
    /// The index in `ids` the game makes.
    pub chosen: usize,
}

impl<S> Conflict<S> {
    /// The recipe ids as a set: the key a choice is kept under.
    pub fn set(&self) -> Vec<String> {
        set_of(&self.ids)
    }
}

fn set_of(ids: &[String]) -> Vec<String> {
    let mut set = ids.to_vec();
    set.sort();
    set.dedup();
    set
}

/// Everything Polymorph remembers.
#[derive(Debug)]
pub struct Choices<S> {
    by_set: HashMap<Vec<String>, String>,
    /// The recipe last picked by hand: Polymorph's selected recipe.
    last: Option<String>,
    /// A recipe a viewer's "+" asked for, taken by the next conflict that has it.
    pending: Option<String>,
    /// Newest first.
    recent: Vec<Conflict<S>>,
    /// A choice changed since [`Choices::take_changed`] last looked: the open grids' results are stale.
    changed: bool,
}

impl<S> Default for Choices<S> {
    fn default() -> Self {
        Self { by_set: HashMap::new(), last: None, pending: None, recent: Vec::new(), changed: false }
    }
}

impl<S: Clone> Choices<S> {
    /// Which of `ids` (the grid's matches, in the game's order, at least two) the game makes, remembering the
    /// conflict for the selector. `same` says whether two cells hold the same kind of item (count aside).
    pub fn choose(
        &mut self,
        size: usize,
        grid: Vec<Option<S>>,
        ids: Vec<String>,
        outputs: Vec<S>,
        same: impl Fn(&S, &S) -> bool,
    ) -> usize {
        let set = set_of(&ids);
        if let Some(wanted) = self.pending.as_ref().filter(|p| ids.contains(p)).cloned() {
            self.pending = None;
            self.by_set.insert(set.clone(), wanted.clone());
            self.last = Some(wanted);
        }
        let position = |id: &String| ids.iter().position(|i| i == id);
        let chosen = self
            .by_set
            .get(&set)
            .and_then(position)
            .or_else(|| self.last.as_ref().and_then(position))
            .unwrap_or(0);
        self.recent.retain(|c| !(c.size == size && same_grid(&c.grid, &grid, &same)));
        self.recent.insert(0, Conflict { size, grid, ids, outputs, chosen });
        self.recent.truncate(RECENT);
        chosen
    }

    /// The conflict recorded for this grid, if its ingredients match more than one recipe.
    pub fn conflict(&self, size: usize, grid: &[Option<S>], same: impl Fn(&S, &S) -> bool) -> Option<&Conflict<S>> {
        self.recent.iter().find(|c| c.size == size && same_grid(&c.grid, grid, &same))
    }

    /// The player picked `id` for the conflict of this grid. False when the grid has no such conflict.
    pub fn select(&mut self, size: usize, grid: &[Option<S>], id: &str, same: impl Fn(&S, &S) -> bool) -> bool {
        let Some(c) = self.recent.iter_mut().find(|c| c.size == size && same_grid(&c.grid, grid, &same)) else {
            return false;
        };
        let Some(index) = c.ids.iter().position(|i| i == id) else { return false };
        c.chosen = index;
        let set = c.set();
        self.by_set.insert(set, id.to_owned());
        self.last = Some(id.to_owned());
        self.pending = None;
        self.changed = true;
        true
    }

    /// A viewer's "+" moved recipe `id` into a grid: make it if that grid's ingredients match others too.
    pub fn prefer(&mut self, id: &str) {
        self.pending = Some(id.to_owned());
    }

    /// Whether a choice changed since the last call.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }
}

fn same_grid<S>(a: &[Option<S>], b: &[Option<S>], same: &impl Fn(&S, &S) -> bool) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|pair| match pair {
            (None, None) => true,
            (Some(x), Some(y)) => same(x, y),
            _ => false,
        })
}
