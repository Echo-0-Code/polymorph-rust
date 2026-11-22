fn find_recipe(
    this: &Recipes,
    context: &LoopContext,
    grid: &[Option<ItemStack>],
    size: usize,
    width: usize,
    height: usize,
    map_scale: &dyn Fn(i32) -> Option<u8>,
) -> ::core::ops::ControlFlow<Option<(&'static str, ItemStack)>> {
    ::core::ops::ControlFlow::Break(crate::crafting::mod_kinds::polymorph_match::pick(
        this, context, grid, size, width, height, map_scale,
    ))
}
