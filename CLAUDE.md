# Polymorph for Horizon: start here

Port of Polymorph (LGPL-3.0-or-later) to Horizon (Rust Minecraft 26.3), as a wasm mod on Horizon's
`crafting_recipe_choice` hook. Read [notes/FINDINGS.md](notes/FINDINGS.md) first.

- Game-independent logic with tests: `mod/src/choice.rs` (which recipe a grid makes), `mod/src/layout.rs`,
  the pick message in `mod/src/engine.rs`. Tests in `mod/src/tests.rs` (no `.rs` outside `src/`: it breaks
  packaging).
- `mod/src/glue.rs`: the hooks (the choice, the selector on the client, the server's side of a pick).
- Keep NOTICE.md, both licence files and the manifest's credits. No session links in commits.
- Never write into a live Horizon instance folder while the game runs.

Build: `HORIZON_SDK_BASE=<mirror> cargo horizon-mod build --wasm` (mirror steps in jei-rust's FINDINGS), `cargo test`.
The hook is new: until a Horizon build has it, check against a Horizon checkout of the PR branch (FINDINGS).
