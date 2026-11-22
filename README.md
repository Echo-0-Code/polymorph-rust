# Polymorph for Horizon

Port of [Polymorph](https://github.com/TheIllusiveC4/Polymorph) (LGPL-3.0-or-later) to
[Horizon](https://github.com/horizon-rust/horizon), the Rust rewrite of Minecraft Java 26.3. Native mod.

When the ingredients in a crafting grid match more than one recipe, a button appears above the result slot.
It opens a row of every result; the one picked is crafted, and stays picked for those ingredients.
JEI's "+" (Echo-0-Code/jei-rust) picks the recipe it moved in.

Start with [CLAUDE.md](CLAUDE.md) and [notes/FINDINGS.md](notes/FINDINGS.md).

Build: `cd mod && cargo test && cargo horizon-mod build` (writes `target/horizon-mod/polymorph.hmod`), then put the
`.hmod` in the instance's `horizon-native-mods` and use the launcher's **Build and play**.
