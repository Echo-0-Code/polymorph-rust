# Findings: Polymorph on Horizon

Polymorph source: 1.2.0+1.21.1, `TheIllusiveC4/Polymorph` @ `a2dc1e5` (common/ is all the logic; the
Forge/NeoForge/Fabric folders are loaders). First checked 2026-11-22 against Horizon `2b77287` (Nightly 2026-11-22).

## Status (2026-11-22): wasm mod waiting on a Horizon PR

Horizon `2b77287` can't let a mod pick among matching crafting recipes (details below). Jan chose an upstream
PR: branch `crafting-recipe-choice` on a fork of horizon-rust/horizon, based on main `b1947c6`, which adds

- hook `crafting_recipe_choice` (both sides, loop context, listen): asked from
  `Recipes::find_with_dimensions` when a grid matches two or more recipes, with the grid (`width` wide), the
  qualified recipe ids, their results and `choice` (0 = first match). The result slot, the take's re-check, its
  remainders, `item_crafted` and the crafter all go through that function, so they all follow the answer.
  With no listener the matcher stops at the first match as before.
- `PlayerRef::crafting_changed(host)` (server): makes the player's open crafting results again (inventory grid,
  crafting table, mod crafting menu) without a grid change, as Polymorph's selection packet calls
  `slotsChanged`.

The mod (`mod/src/glue.rs`): the hook answers from `engine::choose` on both sides. A click in the selector sends
`polymorph:pick` (picked id + the set of matching ids) with `queue_payload_to_server` (the click hook only has a
shared host); only when that is accepted is the pick kept locally (client prediction). `UnsupportedPeer` means the
server has no Polymorph: the selector hides for that server. The server takes a pick only for a set it matched
itself lately (`Choices::recent`), keeps at most `KEPT` (256) sets, and on its next tick calls `crafting_changed`
for every player, since choices are per set of recipes and shared by all players (the hook has no player; Polymorph
keeps one per player). Per-player choices need the player in the hook: noted on upstream PR #20984 (2026-11-24 review).

Checking before the hook is in a build: copy `mod/` elsewhere, point `horizon-mod-api` at the branch's
`crates/horizon-mod-api`, add `[workspace]`, `cargo test`. Packaging (`cargo horizon-mod build --wasm`) needs an
SDK that has the hook, so it waits for a Horizon build with the PR.

Dropped with the mixins: JEI's "+" choosing the recipe it moved in (was a mixin on `transfer_viewer_recipe_id`).
The selector still shows after "+", so the player picks there.

The rest of this file is the investigation that led here, kept for reference.

## Why native, with mixins

- `prepare_item_craft` (server, `contract = "replace"`) looks like the hook for this, but the generated dispatch
  starts with `if result.is_some() { return; }`: mods are only asked when **no** recipe matched. It can fill an
  empty grid's result, never replace a matched one (`native_stations_tests.rs`: "No recipe takes one dirt: the
  result is the mods'"). Every `replace` hook works this way (`horizon-mod-api-gen/src/dispatch.rs`).
- No hook gives the list of matching recipes or lets a mod pick one. So Polymorph needs the native route; a wasm
  mod can't.
- One function decides every crafting grid: `horizon_client::crafting::mod_kinds::Recipes::find_with_dimensions`
  (first match wins). `find`, `find_with`, `craft` call it. Its callers: the client's and the server's result
  slot, the take (`menu/crafting_take.rs`: result, `item_crafted`'s recipe, remainders), LAN/integrated-server
  guests (`game/network/lan_crafting.rs`), the crafter, mod crafting menus, `native_recipes`. A cancellable `HEAD`
  mixin there that scans all recipes and returns the chosen one fixes them all at once, with the right recipe id
  (so remainders and `item_crafted` follow the choice).

## Blocker found by the compile check (2026-11-22)

`crafting.rs`, `crafting/mod_kinds.rs` and `menu*.rs` sit under `crates/horizon-client/src/` but are compiled into
the crate **horizon-client-lib** (`[lib] path = "../horizon-client/src/client_lib.rs"`), which `horizon_client`
re-exports (`pub use horizon_client_lib::*`). That is a crate below the game crate: a mixin there needs
`#[horizon_mod_api_macros::replaceable]` on the function, and `[[native.module]]` only takes files of
`horizon_client` itself.

- `Recipes::find_with_dimensions` has no marker, so the design below can't be built on `2b77287`.
- In menu.rs only `Menu::recompute` is marked. A mixin there could show the chosen result, but the take re-checks
  it with the unmarked `crafting_result_is_current` (first match only, whenever any mod registers a code crafting
  kind: Sophisticated Backpacks does), and remainders and `item_crafted` follow the first match. Not shippable.
- `Recipes` is not `Clone` and the server's table is often the shared `Resources.rest` one, so reordering the
  recipe list from the game crate is not possible either.

So Polymorph needs one change in Horizon: a hook in `find_with_dimensions` that lists every match and lets a mod
pick (or the marker on it). With a hook, Polymorph can be a wasm mod with no mixins.

The rest of this file describes the mixin design, kept for reference.

## Pitfalls (of the mixin design)

- A mixin helper may not have generic (or lifetime) parameters, and `find_with_dimensions` returns
  `Option<(&str, ItemStack)>` borrowed from `self`. With several reference parameters the helper can't elide that
  lifetime. The helper returns `&'static str` instead (covariant, so it coerces), from `polymorph::engine::intern`,
  which keeps one copy of each recipe id.
- `MODIFY_VARIABLE` `LOAD` only takes primitives, and `STORE` refuses a value containing closures, so the
  per-recipe `result` in that loop can't be rewritten; `WRAP_OPERATION` on `assemble_if_matches` is inside a
  closure (`E_UNSUPPORTED_SITE`). Hence `HEAD` + a support module that reimplements the loop.
- Single player is a client plus an integrated server `Game` (`ExecutionRole::IntegratedServer`) in one
  process, the player being a "guest" of the server's LAN hub. The client's result slot shows the server's
  (`craft_result()` returns `remote.craft_result`). Statics in the mod crate are shared by both, so the choice
  made on the client screen is the one the server's matcher reads.
- The server makes a grid's result again only when the grid changes. After a choice nothing changed, so a
  `HEAD` mixin on `lan_tables_tick` (server tick, `tick_guests_inner`) re-makes every guest's inventory grid
  (`guest_craft_result`), crafting table (grid set again, `prepare_item_craft`, `lan_send_table_content`) and mod
  crafting menu (`guest.menu.craft = None`, made again by `lan_mod_crafting_tick` in the same tick).
- The client `Game` has `lan == None`; the helper checks that before taking the "changed" flag, or the client's
  tick could eat it.
- Recipe ids in `Recipes::crafting` are bare file names for vanilla (`oak_planks`) and `ns:path` for mods; JEI
  passes `minecraft:oak_planks`. Compare qualified ids.
- `screen_hovered_item`-style APIs aren't needed: `MenuRef::slot_kinds` marks `CraftingGrid` and `Result`, and
  `slot_boxes` gives each slot's 16x16 item box, so the selector finds any crafting screen (inventory, table,
  crafter, AE2 terminal) without knowing its type.
- `client_session().client_server().integrated_local` tells whether this window hosts the world. On someone
  else's server the choice can't reach their matcher, so the selector stays hidden there.
- Dead-code warnings for everything in `glue.rs` in a plain `cargo build` are normal: `#[horizon_mod]` registers
  the hooks only in the launcher's build.

## Checking the game-side code locally

The launcher compiles `src/game/*` inside copies of the target functions. To catch type and privacy errors before
Jan's 20-30 minute launcher build, inject them by hand into a checkout of the pinned commit and check it:

1. `git worktree add ../hz <horizon commit>` in a Horizon clone.
2. Copy `mod/` elsewhere, point its `horizon-mod-api` at `../hz/crates/horizon-mod-api` (one `mod_api` for both).
3. Paste each helper as a local fn at the start of its target's body with the call (`if let Break(v) = ... {
   return v; }` for cancellable), each `[[native.module]]` as `pub(crate) mod <name> { ... }` at the end of its
   file, add `polymorph = { path = ... }` to `crates/horizon-client/Cargo.toml`.
4. `cargo check -p horizon-client`.

## Not done yet (phase 2)

- Furnaces (per-block choice saved with the block, `FurnaceRecipesWidget`): the cooking lookup is
  `Recipes::cook_index`, first recipe per input. Vanilla has no smelting conflicts; mods do.
- Smithing (`SmithingMenu`, `station_recipes.smithing_result`).
- Choices are kept until the game closes (Polymorph saves the player's one selected recipe).
- A client on another server (dedicated or LAN host with Polymorph) can't choose: needs a payload to the server.
- `/polymorph conflicts` (Polymorph's command listing all conflicts to logs/conflicts.log).
