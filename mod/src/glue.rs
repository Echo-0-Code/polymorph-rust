//! The selector on the client: Polymorph's `PlayerRecipesWidget` over any screen with a crafting grid and a
//! result slot (the inventory, a crafting table, a crafter, a mod's crafting menu such as AE2's terminal).
//!
//! It shows only in a world this game hosts (single player, or the host of a LAN world): the choice is kept in
//! this process, where the integrated server makes the result. A client of another server has no say there.

use std::sync::Mutex;

use mod_api::{
    ClientHost, DrawContext, Flow, Hooks, ItemResource, ItemStack, MenuRef, MenuSlotKind, ScreenMouseButtonPressedPreEvent,
    ScreenRenderPostEvent, ScreenView, Text, TooltipElement,
};

use crate::engine;
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

/// The open screen's grid and result slot, with the conflict recorded for that grid.
struct Shown {
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

fn hosted(host: &dyn ClientHost) -> bool {
    host.client_session().client_server().is_some_and(|s| s.integrated_local)
}

fn shown(host: &dyn ClientHost, screen: &ScreenView<'_>) -> Option<Shown> {
    if screen.id != "inventory" || !hosted(host) {
        return None;
    }
    let menu = host.local_player()?.container_menu(host)?;
    let kinds = menu.slot_kinds(host)?;
    let boxes = menu.slot_boxes(host)?;
    let result = kinds.iter().position(|k| matches!(k, MenuSlotKind::Result))?;
    let cells: Vec<usize> =
        kinds.iter().enumerate().filter(|(_, k)| matches!(k, MenuSlotKind::CraftingGrid)).map(|(i, _)| i).collect();
    let size = (1..=3).find(|s| s * s == cells.len())?;
    let slot = boxes.get(result).copied().flatten()?;
    let grid = cells.iter().map(|&s| menu.slot(host, s)).collect::<Option<Vec<_>>>()?;
    let (ids, outputs, chosen) = engine::with(|c| {
        c.conflict(size, &grid, engine::same).map(|k| (k.ids.clone(), k.outputs.clone(), k.chosen))
    })?;
    if ids.len() < 2 {
        return None;
    }
    Some(Shown { menu, size, grid, button: layout::button(slot), ids, outputs, chosen })
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
                engine::with(|c| c.select(s.size, &s.grid, &id, engine::same));
            }
        }
        Flow::Cancel
    }
}
