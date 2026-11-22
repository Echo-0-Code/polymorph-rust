fn server_tick(game: &mut Game<'_>) {
    if game.lan.is_some() && polymorph::engine::take_changed() {
        crate::game::lan_crafting::polymorph_refresh::refresh(game);
    }
}
