# Polymorph for Horizon: start here

Port of Polymorph (LGPL-3.0-or-later) to Horizon (Rust Minecraft 26.3), as a native mod with native-route mixins.
Read [notes/FINDINGS.md](notes/FINDINGS.md) first.

- Game-independent logic with tests: `mod/src/choice.rs` (which recipe a grid makes), `mod/src/layout.rs`.
  Tests in `mod/src/tests.rs` (no `.rs` outside `src/`: it breaks packaging).
- `mod/src/engine.rs`: the plain-API functions the game-side code calls. `mod/src/glue.rs`: the selector (client).
- `mod/src/game/`: compiled into the game by the launcher (`[[native.mixin]]`/`[[native.module]]` in
  `mod/horizon-mod.toml`), never part of the crate. They are pinned to `[native] horizon`: reread their target
  functions whenever that commit moves.
- Keep NOTICE.md, both licence files and the manifest's credits. No session links in commits.
- Never write into a live Horizon instance folder while the game runs.

Check the game-side code before shipping: inject it into a checkout of the pinned Horizon commit and
`cargo check -p horizon-client` (steps in FINDINGS).
