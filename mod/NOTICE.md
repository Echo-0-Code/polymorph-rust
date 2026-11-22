# Notice

Polymorph for Horizon is a port of **Polymorph** by TheIllusiveC4 / Illusive Soulworks
(https://github.com/TheIllusiveC4/Polymorph), ported from Polymorph 1.2.0+1.21.1, commit
`a2dc1e5a91e5694943d84e32caa729cd477098e3`.

Polymorph is licensed under the GNU Lesser General Public License, version 3 or later (`LICENSE`), which builds on
the GNU General Public License, version 3 (`LICENSE-GPL3`). This port is a derivative work and is licensed the same
way: LGPL-3.0-or-later.

From Polymorph, unchanged:
- `assets/polymorph/textures/gui/sprites/*.png` (the selector and result buttons)
- `icon.png` (Polymorph's `polymorph_icon.png`)

The Rust code is new, written after Polymorph's Java (`AbstractRecipeData`, `AbstractRecipesWidget`,
`SelectionWidget`, `OutputWidget`, `PolymorphJeiPlugin`), with Polymorph's names where they apply.

Runs on Horizon (https://github.com/horizon-rust/horizon), whose game code the files in `src/game/` are compiled
into by the launcher.
