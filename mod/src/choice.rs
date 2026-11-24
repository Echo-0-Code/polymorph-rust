//! Polymorph's recipe choice, without the game: which of the recipes that match one crafting grid is made.
//!
//! Polymorph (`AbstractRecipeData.getRecipe`) keeps one selected recipe per player and makes it while it is
//! among the grid's matches, else the first match. Here the choice is kept per set of matching recipes, so a
//! conflict chosen once stays chosen when the same ingredients come back ("as long as the ingredients don't
//! change" in Polymorph's words, and also after they did). The last recipe picked by hand is tried next, then the
//! first match, as in Polymorph.
//!
//! Polymorph keeps the choices per player ([`Players`]): one player's pick never changes what another makes.
//!
//! The game reaches this through [`crate::engine`] (its `crafting_recipe_choice` hook); everything here is plain data so it can be tested.

use std::collections::{HashMap, VecDeque};

/// How many grids with a conflict are remembered for the selector (the open screen asks for its own grid).
pub const RECENT: usize = 16;
/// Polymorph shows at most 15 results (`AbstractRecipeData.getRecipe`, `recipesList.size() < 15`).
pub const MAX_SHOWN: usize = 15;
/// How many sets of recipes keep a pick: the oldest pick is forgotten past this, so a client sending made-up
/// sets can't grow the server's memory.
pub const KEPT: usize = 256;

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
    /// The sets in `by_set`, oldest first.
    order: VecDeque<Vec<String>>,
    /// Newest first.
    recent: Vec<Conflict<S>>,
    /// A choice changed since [`Choices::take_changed`] last looked: the open grids' results are stale.
    changed: bool,
}

impl<S> Default for Choices<S> {
    fn default() -> Self {
        Self { by_set: HashMap::new(), order: VecDeque::new(), last: None, recent: Vec::new(), changed: false }
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
        self.keep(set, id);
        true
    }

    /// A pick heard from elsewhere (the server hears the client's): `id` is made whenever the matching
    /// recipes are `set`. Taken only for a set this side matched itself lately (one of the recent conflicts)
    /// that has `id`, so a client can't make the server keep sets it never crafted. False otherwise.
    pub fn pick(&mut self, set: &[String], id: &str) -> bool {
        let set = set_of(set);
        let mut known = false;
        for c in self.recent.iter_mut().filter(|c| set_of(&c.ids) == set) {
            if let Some(index) = c.ids.iter().position(|i| i == id) {
                c.chosen = index;
                known = true;
            }
        }
        if known {
            self.keep(set, id);
        }
        known
    }

    /// Remembers `id` for `set`, forgetting the oldest set past [`KEPT`].
    fn keep(&mut self, set: Vec<String>, id: &str) {
        if self.by_set.insert(set.clone(), id.to_owned()).is_none() {
            self.order.push_back(set);
            while self.order.len() > KEPT {
                if let Some(old) = self.order.pop_front() {
                    self.by_set.remove(&old);
                }
            }
        }
        self.last = Some(id.to_owned());
        self.changed = true;
    }

    /// Whether a choice changed since the last call.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }
}

/// Every player's choices, by uuid.
#[derive(Debug)]
pub struct Players<S> {
    by_player: HashMap<u128, Choices<S>>,
}

impl<S> Default for Players<S> {
    fn default() -> Self {
        Self { by_player: HashMap::new() }
    }
}

impl<S: Clone> Players<S> {
    /// [`Choices::choose`] for the grid of `player`; a grid nobody crafts in (`None`: a crafter block, a
    /// mod's lookup) makes the first match, as vanilla, and is not remembered.
    pub fn choose(
        &mut self,
        player: Option<u128>,
        size: usize,
        grid: Vec<Option<S>>,
        ids: Vec<String>,
        outputs: Vec<S>,
        same: impl Fn(&S, &S) -> bool,
    ) -> usize {
        match player {
            Some(player) => self.of(player).choose(size, grid, ids, outputs, same),
            None => 0,
        }
    }

    /// `player`'s choices.
    pub fn of(&mut self, player: u128) -> &mut Choices<S> {
        self.by_player.entry(player).or_default()
    }

    /// The player left: their choices go.
    pub fn forget(&mut self, player: u128) {
        self.by_player.remove(&player);
    }
}

/// Server: the list of matching recipes each player should be sent (Polymorph's `SPacketRecipesList`). A grid
/// is matched many times per change, so a list goes out only when it differs from the one the player was last sent.
#[derive(Debug)]
pub struct Outbox<S> {
    sent: HashMap<u128, Conflict<S>>,
    waiting: Vec<(u128, Conflict<S>)>,
}

impl<S> Default for Outbox<S> {
    fn default() -> Self {
        Self { sent: HashMap::new(), waiting: Vec::new() }
    }
}

impl<S: Clone> Outbox<S> {
    /// `player`'s grid matches `c.ids`, and `c.chosen` is made.
    pub fn offer(&mut self, player: u128, c: Conflict<S>, same: impl Fn(&S, &S) -> bool) {
        let unchanged = self.sent.get(&player).is_some_and(|s| {
            s.size == c.size && s.ids == c.ids && s.chosen == c.chosen && same_grid(&s.grid, &c.grid, &same)
        });
        if unchanged {
            return;
        }
        self.sent.insert(player, c.clone());
        self.waiting.retain(|w| w.0 != player);
        self.waiting.push((player, c));
    }

    /// The lists to send now, oldest first.
    pub fn take(&mut self) -> Vec<(u128, Conflict<S>)> {
        std::mem::take(&mut self.waiting)
    }

    /// The player left.
    pub fn forget(&mut self, player: u128) {
        self.sent.remove(&player);
        self.waiting.retain(|w| w.0 != player);
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
