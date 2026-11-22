# Polymorph for Horizon

Port of [Polymorph](https://github.com/TheIllusiveC4/Polymorph) (LGPL-3.0-or-later) to
[Horizon](https://github.com/horizon-rust/horizon), the Rust rewrite of Minecraft Java 26.3. Wasm mod.

When the ingredients in a crafting grid match more than one recipe, a button appears above the result slot.
It opens a row of every result; the one picked is crafted, and stays picked for those ingredients. It works in
the inventory, crafting tables and mod crafting menus (AE2's crafting terminal).

It needs Horizon's `crafting_recipe_choice` hook and `PlayerRef::crafting_changed`, which are not in a Horizon
build yet (see [notes/FINDINGS.md](notes/FINDINGS.md)). Start with [CLAUDE.md](CLAUDE.md).

Build: `cd mod && cargo test && cargo horizon-mod build --wasm` (writes `target/horizon-mod/polymorph.hmod`), then
put the `.hmod` in the instance's `horizon-mods`.
