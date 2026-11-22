// Polymorph's matcher, compiled into the game as module `polymorph_match` of
// `crates/horizon-client/src/crafting/mod_kinds.rs` (a `[[native.module]]`). `super::` is that module, which
// has `Recipes::find_with_dimensions`, and through its `use super::*` the crafting module's items.
//
// `pick` is `find_with_dimensions` (`RecipeManager.getRecipeFor`) with Polymorph's choice
// (`AbstractRecipeData.getRecipe`): every recipe that matches the grid, then the one the player chose, else the
// first. Recipes are tried as the game tries them, so with one match the result is the game's own.

/// The chosen recipe's id and result; None when no recipe makes anything from the grid.
pub fn pick(
    recipes: &super::Recipes,
    context: &super::LoopContext,
    grid: &[Option<super::ItemStack>],
    size: usize,
    width: usize,
    height: usize,
    map_scale: &dyn Fn(i32) -> Option<u8>,
) -> Option<(&'static str, super::ItemStack)> {
    let vanilla = super::Input::of(grid, size);
    if vanilla.is_none() && !recipes.mod_crafting {
        return None;
    }
    let input = recipes
        .mod_crafting
        .then(|| super::snapshot_dimensions(grid, size, width, height))
        .flatten();
    let ctx = super::Context {
        recipes,
        context,
        map_scale,
    };
    // Each match that makes something (Polymorph skips one whose result is empty).
    let mut found: Vec<(&str, super::ItemStack)> = Vec::new();
    for (id, recipe) in &recipes.crafting {
        let result = match recipe {
            super::Recipe::Mod(recipe) => match input.as_ref() {
                Some(input) => recipe.assemble_owned(input.clone()),
                None => None,
            },
            _ => match vanilla.as_ref() {
                Some(input) => super::Recipes::assemble_if_matches(&ctx, input, recipe),
                None => None,
            },
        };
        match result {
            Some(Some(output)) => found.push((id.as_str(), output)),
            // The game's own matcher stops at a match that makes nothing.
            Some(None) if found.is_empty() => return None,
            _ => {}
        }
    }
    if found.len() < 2 {
        return found
            .pop()
            .map(|(id, output)| (polymorph::engine::intern(id), output));
    }
    let api_grid: Option<Vec<Option<mod_api::ItemStack>>> = (0..size * size)
        .map(|i| match grid.get(i) {
            Some(Some(stack)) => super::to_api(stack).map(Some),
            _ => Some(None),
        })
        .collect();
    let api_outputs: Option<Vec<mod_api::ItemStack>> = found
        .iter()
        .take(polymorph::choice::MAX_SHOWN)
        .map(|(_, output)| super::to_api(output))
        .collect();
    let chosen = match (api_grid, api_outputs) {
        (Some(api_grid), Some(api_outputs)) => {
            let ids = found
                .iter()
                .take(api_outputs.len())
                .map(|(id, _)| (*id).to_owned())
                .collect();
            polymorph::engine::choose(size, api_grid, ids, api_outputs)
        }
        // A stack the API can't carry: the game's own first match, with no selector.
        _ => 0,
    };
    let (id, output) = found.swap_remove(chosen.min(found.len() - 1));
    Some((polymorph::engine::intern(id), output))
}
