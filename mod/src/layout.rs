//! Where Polymorph draws its selector, from its widgets (`AbstractRecipesWidget`, `SelectionWidget`,
//! `OpenSelectionButton`, `OutputWidget`), in GUI pixels. Rectangles are `[x, y, w, h]`.

/// The 16 by 16 button above the result slot (`BUTTON_X_OFFSET` 0, `BUTTON_Y_OFFSET` -22).
pub const BUTTON: f32 = 16.0;
/// Each result in the list is a 25 by 25 button, the item 4 pixels in.
pub const OUTPUT: f32 = 25.0;
pub const ITEM_INSET: f32 = 4.0;

/// The button for a result slot whose item sits at `slot` (`[x, y, 16, 16]`).
pub fn button(slot: [f32; 4]) -> [f32; 4] {
    [slot[0], slot[1] - 22.0, BUTTON, BUTTON]
}

/// The list's buttons for `n` results, centred over the button: the row starts at the button plus
/// `WIDGET_X_OFFSET` -4, `WIDGET_Y_OFFSET` -26, moved left by 25 per pair, and 13 back for an even count.
pub fn outputs(button: [f32; 4], n: usize) -> Vec<[f32; 4]> {
    let x = button[0] - 4.0;
    let y = button[1] - 26.0;
    let mut offset = -25.0 * (n / 2) as f32;
    if n % 2 == 0 {
        offset += 13.0;
    }
    (0..n).map(|i| [x + offset + OUTPUT * i as f32, y, OUTPUT, OUTPUT]).collect()
}

pub fn inside(r: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= r[0] && at[0] < r[0] + r[2] && at[1] >= r[1] && at[1] < r[1] + r[3]
}
