//! The selector on the client: Polymorph's `PlayerRecipesWidget` over any screen with a crafting grid and a
//! result slot (the inventory, a crafting table, a crafter, a mod's crafting menu such as AE2's terminal).
//!
//! A pick is kept here for the client's prediction and sent to the server ([`engine::PICK`]), whose handler
//! keeps it and has that player's crafting results made again on its next tick (each player keeps their own
//! choices, as in Polymorph). A server without Polymorph refuses the pick: nothing changes and the selector
//! hides there. On both sides the game asks `crafting_recipe_choice` whenever a grid matches more than one recipe.

use std::sync::{Arc, Mutex};

use mod_api::{
    ClientHost, CraftingRecipeChoiceEvent, DrawContext, Flow, Hooks, IPayloadContext, IPayloadHandler, ItemResource,
    ItemStack, LoopContext, MenuRef, MenuSlotKind, PayloadBytes, PayloadHandleError, PayloadId, PayloadSendError, PlayerLoggedOutEvent, Registrar,
    Identifier, RegistrationContext, ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent, ScreenView, ServerHost,
    ServerTickEvent, Text, TooltipElement,
};

use crate::engine::{self, Listed, Pick};
use crate::layout::{self, ITEM_INSET};

const SELECTOR: &str = "polymorph:gui/sprites/selector_button";
const SELECTOR_HOVERED: &str = "polymorph:gui/sprites/selector_button_highlighted";
const OUTPUT: &str = "polymorph:gui/sprites/output_button";
const OUTPUT_HOVERED: &str = "polymorph:gui/sprites/output_button_highlighted";
const CURRENT: &str = "polymorph:gui/sprites/current_output";
const CURRENT_HOVERED: &str = "polymorph:gui/sprites/current_output_highlighted";

/// The menu whose list is open (`SelectionWidget.active`).
static OPEN: Mutex<Option<MenuRef>> = Mutex::new(None);

fn open() -> std::sync::MutexGuard<'static, Option<MenuRef>> {
    OPEN.lock().unwrap_or_else(|e| e.into_inner())
}

/// Client: the server this game is connected to answered that it has no Polymorph (an optional channel it
/// lacks). Its matcher makes the first match whatever is picked here, so the selector stays hidden there.
static NO_POLYMORPH: Mutex<Option<ServerKey>> = Mutex::new(None);

type ServerKey = (Option<String>, Option<String>, Option<String>);

fn server_key(host: &dyn ClientHost) -> Option<ServerKey> {
    host.client_session().client_server().map(|s| (s.name, s.address, s.save_folder))
}

fn no_polymorph() -> std::sync::MutexGuard<'static, Option<ServerKey>> {
    NO_POLYMORPH.lock().unwrap_or_else(|e| e.into_inner())
}

/// Server: the players whose pick arrived since the last tick; their open grids are made again then.
static STALE: Mutex<Vec<u128>> = Mutex::new(Vec::new());

fn stale() -> std::sync::MutexGuard<'static, Vec<u128>> {
    STALE.lock().unwrap_or_else(|e| e.into_inner())
}

/// `CPacketPlayerRecipeSelection.handle` on the server: the pick is kept, and the player's open grids are
/// made again on the next tick (`ServerPlayer.containerMenu.slotsChanged`).
struct PickHandler;

impl IPayloadHandler for PickHandler {
    fn handle(&self, payload: PayloadBytes, context: &mut dyn IPayloadContext) -> Result<(), PayloadHandleError> {
        let Some(pick) = Pick::decode(&payload.0) else { return Ok(()) };
        let Ok(player) = context.player() else { return Ok(()) };
        let uuid = player.uuid();
        if engine::with(uuid, |c| c.pick(&pick.set, &pick.id)) {
            let mut stale = stale();
            if !stale.contains(&uuid) {
                stale.push(uuid);
            }
        }
        Ok(())
    }
}

/// `SPacketRecipesList.handle` on the client: the server's list for this player's grid, which the selector shows
/// while the screen's grid is still that one.
struct ListHandler;

impl IPayloadHandler for ListHandler {
    fn handle(&self, payload: PayloadBytes, _context: &mut dyn IPayloadContext) -> Result<(), PayloadHandleError> {
        if let Some(list) = Listed::decode(&payload.0) {
            *engine::FROM_SERVER.lock().unwrap_or_else(|e| e.into_inner()) = Some(list);
        }
        Ok(())
    }
}

/// The server's list, when it is for `grid` (`size` wide): its ids, outputs and chosen index.
fn listed_for(host: &dyn ClientHost, size: usize, grid: &[Option<ItemStack>]) -> Option<(Vec<String>, Vec<ItemStack>, usize)> {
    let list = engine::FROM_SERVER.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
    let cells: Vec<Option<String>> = grid.iter().map(|c| c.as_ref().map(|s| s.item().id(host))).collect();
    if list.size != size || list.cells != cells {
        return None;
    }
    let outputs = list
        .outputs
        .iter()
        .map(|(item, count)| ItemStack::new(host.item_key(&Identifier::parse(item).ok()?)?, *count))
        .collect::<Option<Vec<_>>>()?;
    Some((list.ids, outputs, list.chosen))
}

/// The open screen's grid and result slot, with the conflict recorded for that grid.
struct Shown {
    player: u128,
    menu: MenuRef,
    size: usize,
    grid: Vec<Option<ItemStack>>,
    button: [f32; 4],
    ids: Vec<String>,
    outputs: Vec<ItemStack>,
    chosen: usize,
}

impl Shown {
    fn outputs_at(&self) -> Vec<[f32; 4]> {
        layout::outputs(self.button, self.outputs.len())
    }
}

fn shown(host: &dyn ClientHost, screen: &ScreenView<'_>) -> Option<Shown> {
    if screen.id != "inventory" {
        return None;
    }
    let key = server_key(host);
    if no_polymorph().as_ref().is_some_and(|k| Some(k) == key.as_ref()) {
        return None;
    }
    let player = host.local_player()?;
    let uuid = player.uuid();
    let menu = player.container_menu(host)?;
    let kinds = menu.slot_kinds(host)?;
    let boxes = menu.slot_boxes(host)?;
    let result = kinds.iter().position(|k| matches!(k, MenuSlotKind::Result))?;
    let cells: Vec<usize> =
        kinds.iter().enumerate().filter(|(_, k)| matches!(k, MenuSlotKind::CraftingGrid)).map(|(i, _)| i).collect();
    let size = (1..=3).find(|s| s * s == cells.len())?;
    let slot = boxes.get(result).copied().flatten()?;
    let grid = cells.iter().map(|&s| menu.slot(host, s)).collect::<Option<Vec<_>>>()?;
    // In single player the server's matcher runs in this process and keeps the player's conflicts here; a
    // client of another machine has only the server's list.
    let (ids, outputs, chosen) = engine::with(uuid, |c| {
        c.conflict(size, &grid, engine::same).map(|k| (k.ids.clone(), k.outputs.clone(), k.chosen))
    })
    .or_else(|| listed_for(host, size, &grid))?;
    if ids.len() < 2 {
        return None;
    }
    Some(Shown { player: uuid, menu, size, grid, button: layout::button(slot), ids, outputs, chosen })
}

fn sprite_or(draw: &mut dyn DrawContext, name: &str, side: f32, rect: [f32; 4], fallback: u32) {
    if !draw.sprite(name, [0.0, 0.0, side, side], rect, None) {
        draw.fill(rect, 0xFF373737);
        draw.fill([rect[0] + 1.0, rect[1] + 1.0, rect[2] - 2.0, rect[3] - 2.0], fallback);
    }
}

fn tooltip(host: &dyn ClientHost, stack: &ItemStack) -> Vec<Text> {
    let mut lines: Vec<Text> = ItemResource::of(stack)
        .tooltip(host, false, false)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| match e {
            TooltipElement::Text(t, _) => Some(t),
            _ => None,
        })
        .collect();
    if lines.is_empty() {
        lines.push(Text::literal(host.hover_name(stack)));
    }
    lines
}

#[derive(Default)]
struct Polymorph;

#[mod_api::horizon_mod]
impl Hooks for Polymorph {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        r.payload_handlers().registrar("1").optional().play_to_server(PayloadId(engine::PICK.into()), Arc::new(PickHandler))
            .play_to_client(PayloadId(engine::LIST.into()), Some(Arc::new(ListHandler)));
    }

    /// `RecipeManager.getRecipeFor` with Polymorph's choice, on whichever side matches the grid.
    fn crafting_recipe_choice(&self, _context: &dyn LoopContext, event: &mut CraftingRecipeChoiceEvent<'_>) {
        let width = event.width as usize;
        event.choice = engine::choose(event.player, width, event.grid, event.recipes, event.results) as u32;
    }

    fn server_tick(&self, host: &mut dyn ServerHost, _event: &mut ServerTickEvent<'_>) {
        for (uuid, c) in engine::with_outbox(|o| o.take()) {
            let Some(player) = host.player(uuid) else { continue };
            let item = |s: &ItemStack| s.item().id(host);
            let list = Listed {
                size: c.size,
                cells: c.grid.iter().map(|cell| cell.as_ref().map(item)).collect(),
                chosen: c.chosen,
                outputs: c.outputs.iter().map(|s| (item(s), s.count())).collect(),
                ids: c.ids,
            };
            // A client without Polymorph (an optional channel it lacks) refuses it; nothing to do then.
            let _ = host.send_payload_to_players(&[player], &PayloadId(engine::LIST.into()), PayloadBytes(list.encode()));
        }
        let players = std::mem::take(&mut *stale());
        for uuid in players {
            if let Some(player) = host.player(uuid) {
                player.crafting_changed(host);
            }
        }
    }

    fn player_logged_out(&self, _host: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        engine::forget(event.player);
        stale().retain(|&p| p != event.player);
    }

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        let Some(s) = shown(host, event.screen) else {
            *open() = None;
            return;
        };
        let mouse = event.screen.mouse;
        let draw = &mut *event.draw;
        let hover = layout::inside(s.button, mouse);
        sprite_or(draw, if hover { SELECTOR_HOVERED } else { SELECTOR }, layout::BUTTON, s.button, 0xFFA0A0A0);
        let is_open = {
            let mut o = open();
            if o.is_some_and(|m| m != s.menu) {
                *o = None;
            }
            o.is_some()
        };
        if !is_open {
            return;
        }
        let mut hovered = None;
        for (i, rect) in s.outputs_at().into_iter().enumerate() {
            let over = layout::inside(rect, mouse);
            let name = match (i == s.chosen, over) {
                (true, true) => CURRENT_HOVERED,
                (true, false) => CURRENT,
                (false, true) => OUTPUT_HOVERED,
                (false, false) => OUTPUT,
            };
            sprite_or(draw, name, layout::OUTPUT, rect, if i == s.chosen { 0xFF3F9F3F } else { 0xFF8B8B8B });
            let stack = &s.outputs[i];
            let count = (stack.count() > 1).then(|| stack.count().to_string());
            draw.gui_draw_item_with(stack, [rect[0] + ITEM_INSET, rect[1] + ITEM_INSET], count.as_deref());
            if over {
                hovered = Some(i);
            }
        }
        if let Some(i) = hovered {
            let _ = draw.gui_tooltip(&tooltip(host, &s.outputs[i]), [mouse[0] + 8.0, mouse[1] - 12.0]);
        }
    }

    fn screen_mouse_button_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
        let Some(s) = shown(host, event.screen) else {
            *open() = None;
            return Flow::Continue;
        };
        let at = event.screen.mouse;
        let mut o = open();
        let was_open = o.is_some_and(|m| m == s.menu);
        // `AbstractRecipesWidget.mouseClicked`: the button toggles the list, a result in the open list is
        // picked, and any other click closes the open list and is taken.
        if event.button == 0 && layout::inside(s.button, at) {
            *o = if was_open { None } else { Some(s.menu) };
            return Flow::Cancel;
        }
        if !was_open {
            return Flow::Continue;
        }
        *o = None;
        drop(o);
        if matches!(event.button, 0 | 1) {
            if let Some(i) = s.outputs_at().iter().position(|r| layout::inside(*r, at)) {
                let id = s.ids[i].clone();
                let set = s.ids.clone();
                // Kept here (the client's prediction) only once the server has taken it, so the two agree.
                let pick = Pick { id: id.clone(), set };
                match host.queue_payload_to_server(&PayloadId(engine::PICK.into()), PayloadBytes(pick.encode())) {
                    Ok(_) => {
                        engine::with(s.player, |c| c.select(s.size, &s.grid, &id, engine::same));
                        if let Some(list) = engine::FROM_SERVER.lock().unwrap_or_else(|e| e.into_inner()).as_mut().filter(|l| l.ids == s.ids) {
                            list.chosen = i;
                        }
                    }
                    Err(PayloadSendError::UnsupportedPeer(_)) => *no_polymorph() = server_key(host),
                    Err(e) => eprintln!("[polymorph] pick not sent: {e:?}"),
                }
            }
        }
        Flow::Cancel
    }
}
