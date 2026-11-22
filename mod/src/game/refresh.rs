// Polymorph's refresh, compiled into the game as module `polymorph_refresh` of
// `crates/horizon-client/src/game/network/lan_crafting.rs` (a `[[native.module]]`); `super::` is that module.
//
// A choice changes what a grid makes without changing the grid, and the server makes a grid's result again
// only when the grid changes (`slotsChanged`). So after a choice every player's open crafting grids are made
// again here, as the game does after a change, and sent: the inventory's grid, a crafting table's, and a mod
// crafting menu's (AE2's crafting terminal), whose result the next `lan_mod_crafting_tick` makes again.

pub fn refresh(game: &mut super::Game<'_>) {
    let Some(host) = game.lan.as_ref() else {
        return;
    };
    let conns: Vec<u32> = host
        .hub
        .guests
        .iter()
        .filter(|g| g.placed)
        .map(|g| g.conn)
        .collect();
    for conn in conns {
        game.guest_craft_result(conn);
        table(game, conn);
        mod_menu(game, conn);
    }
}

/// The open crafting table: its grid set again (`CraftingMenu.slotsChanged`), the mods asked as after a
/// click, the whole menu sent.
fn table(game: &mut super::Game<'_>, conn: u32) {
    let Some(host) = game.lan.as_mut() else {
        return;
    };
    let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else {
        return;
    };
    let Some((window, state, mut menu, pos)) = host
        .menus
        .get_mut(&conn)
        .and_then(|play| play.crafting.table.take())
    else {
        return;
    };
    let grid: Vec<Option<super::ItemStack>> = (1..10).map(|s| menu.slot(s).cloned()).collect();
    let _ = game.guest_menu_op(gi, &mut menu, |menu, env| {
        for (k, stack) in grid.into_iter().enumerate() {
            menu.set_slot(env, 1 + k, stack);
        }
    });
    if let Some(guest) = game.lan.as_ref().map(|host| &host.hub.guests[gi]) {
        let (uuid, name) = (guest.uuid, guest.name.clone());
        let mut result = menu.slot(0).cloned();
        game.native_prepare_item_craft(uuid, &name, Some(pos), &mut result);
        let env = game.menu_env();
        menu.set_slot(&env, 0, result);
    }
    let Some(host) = game.lan.as_mut() else {
        return;
    };
    *host.menus.entry(conn).or_default().crafting.table = Some((window, state, menu, pos));
    game.lan_send_table_content(conn);
}

/// A mod crafting menu: forget the result it was shown, so the next tick makes it again.
fn mod_menu(game: &mut super::Game<'_>, conn: u32) {
    let Some(host) = game.lan.as_mut() else {
        return;
    };
    let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else {
        return;
    };
    host.hub.guests.update(gi, |g| {
        if let Some(m) = g.menu.as_mut() {
            m.craft = None;
        }
    });
}
