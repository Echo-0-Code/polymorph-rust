//! Polymorph for Horizon, a native mod.
//!
//! A port of Polymorph (TheIllusiveC4 / Illusive Soulworks, LGPL-3.0-or-later): when the ingredients in a crafting
//! grid match more than one recipe, a button above the result lists every result, and the one picked is made.
//!
//! - [`choice`]: which recipe a grid makes, remembered per set of matching recipes (game-independent, tested).
//! - [`layout`]: where Polymorph's button and list go (tested).
//! - [`engine`]: what the game's own code calls. Horizon's `prepare_item_craft` hook can only fill an empty
//!   result, so the choice goes into the game's matcher itself through the native route: the mixins and modules
//!   in `src/game/` (see `horizon-mod.toml`), compiled into the game by the launcher.
//! - `glue`: the selector on the crafting screens.

pub mod choice;
pub mod engine;
pub mod layout;

mod glue;

#[cfg(test)]
mod tests;
