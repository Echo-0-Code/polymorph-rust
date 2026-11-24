//! Polymorph's state in this game, and the message a client's pick travels in.
//!
//! The hook `crafting_recipe_choice` asks [`choose`] on both sides: the client's answer is its prediction,
//! the server's is what is made. A pick on the client screen is kept here at once (so the prediction follows)
//! and sent to the server as [`Pick`], whose handler keeps it there too. As in Polymorph, each player keeps
//! their own choices (the hook names the player whose grid it is); a grid matched on nobody's behalf (a
//! crafter block, a mod's lookup) makes the first match, as vanilla.

use std::sync::{Mutex, MutexGuard};

use mod_api::ItemStack;

use crate::choice::{Choices, Conflict, Outbox, Players};

/// The client's pick, sent to the server (`play_to_server`).
pub const PICK: &str = "polymorph:pick";
/// The server's list of a player's matching recipes, sent to that player (`play_to_client`, Polymorph's
/// `SPacketRecipesList`): a client on another machine never matches the grid itself, so this is how its selector
/// learns the choices.
pub const LIST: &str = "polymorph:list";

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
    with_outbox(|o| o.forget(player));
}

/// Server: the lists waiting to be sent, and what each player was last sent.
static OUTBOX: Mutex<Option<Outbox<ItemStack>>> = Mutex::new(None);

pub(crate) fn with_outbox<R>(f: impl FnOnce(&mut Outbox<ItemStack>) -> R) -> R {
    let mut guard = OUTBOX.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Outbox::default))
}

/// Client: the last list the server sent.
pub(crate) static FROM_SERVER: Mutex<Option<Listed>> = Mutex::new(None);

/// The server's list for one of a player's grids: the grid (`size` wide, each cell's item id), the recipe ids in
/// the game's order, what each makes (item id and count; components are not sent) and the index made.
#[derive(Clone, Debug, PartialEq)]
pub struct Listed {
    pub size: usize,
    pub cells: Vec<Option<String>>,
    pub chosen: usize,
    pub ids: Vec<String>,
    pub outputs: Vec<(String, u32)>,
}

impl Listed {
    /// Line 1: size, chosen; line 2: the cells, tab-separated (empty for an empty cell); then one
    /// `id item count` line per recipe, tab-separated.
    pub fn encode(&self) -> Vec<u8> {
        let cells: Vec<&str> = self.cells.iter().map(|c| c.as_deref().unwrap_or("")).collect();
        let mut text = format!("{} {}\n{}", self.size, self.chosen, cells.join("\t"));
        for (id, (item, count)) in self.ids.iter().zip(&self.outputs) {
            text.push_str(&format!("\n{id}\t{item}\t{count}"));
        }
        text.into_bytes()
    }

    pub fn decode(bytes: &[u8]) -> Option<Listed> {
        let text = std::str::from_utf8(bytes).ok()?;
        let mut lines = text.split('\n');
        let (size, chosen) = lines.next()?.split_once(' ')?;
        let (size, chosen): (usize, usize) = (size.parse().ok()?, chosen.parse().ok()?);
        let cells: Vec<Option<String>> =
            lines.next()?.split('\t').map(|c| (!c.is_empty()).then(|| c.to_owned())).collect();
        if !(1..=3).contains(&size) || cells.len() != size * size {
            return None;
        }
        let (mut ids, mut outputs) = (Vec::new(), Vec::new());
        for line in lines {
            let mut parts = line.split('\t');
            let (id, item, count) = (parts.next()?, parts.next()?, parts.next()?.parse().ok()?);
            if parts.next().is_some() || id.is_empty() || item.is_empty() {
                return None;
            }
            ids.push(id.to_owned());
            outputs.push((item.to_owned(), count));
        }
        (ids.len() >= 2 && ids.len() <= 64 && chosen < ids.len())
            .then_some(Listed { size, cells, chosen, ids, outputs })
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
        ids.clone(),
        outputs.to_vec(),
        same,
    );
    if let Some(player) = player {
        let c = Conflict { size: width, grid: grid.to_vec(), ids, outputs: outputs.to_vec(), chosen };
        with_outbox(|o| o.offer(player, c, same));
    }
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
