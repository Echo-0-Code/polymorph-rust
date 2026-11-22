//! Polymorph for Horizon, a wasm mod.
//!
//! A port of Polymorph (TheIllusiveC4 / Illusive Soulworks, LGPL-3.0-or-later): when the ingredients in a crafting
//! grid match more than one recipe, a button above the result lists every result, and the one picked is made.
//!
//! - [`choice`]: which recipe a grid makes, remembered per set of matching recipes (game-independent, tested).
//! - [`layout`]: where Polymorph's button and list go (tested).
//! - [`engine`]: the choices kept in this game, asked by the `crafting_recipe_choice` hook, and the message a
//!   client's pick travels to the server in.
//! - `glue`: the hooks: the choice, the selector on the crafting screens, the server's side of a pick.

pub mod choice;
pub mod engine;
pub mod layout;

mod glue;

#[cfg(test)]
mod tests;
