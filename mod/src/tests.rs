//! Polymorph's choice and layout, without the game. A stack here is an item name.

use crate::choice::{Choices, MAX_SHOWN, RECENT};
use crate::engine::{Pick, qualified};
use crate::layout;

type S = &'static str;

fn same(a: &S, b: &S) -> bool {
    a == b
}

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn grid(v: &[Option<S>]) -> Vec<Option<S>> {
    v.to_vec()
}

const LOGS: [Option<S>; 4] = [Some("log"), None, None, None];

#[test]
fn first_match_until_one_is_picked() {
    let mut c: Choices<S> = Choices::default();
    let pick = c.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same);
    assert_eq!(pick, 0, "Polymorph makes the first match by default");
    assert!(c.select(2, &LOGS, "b:planks", same));
    assert!(c.take_changed(), "a pick makes the open results stale");
    assert!(!c.take_changed());
    let pick = c.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same);
    assert_eq!(pick, 1);
}

#[test]
fn a_pick_stays_with_its_set_of_recipes() {
    let mut c: Choices<S> = Choices::default();
    c.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same);
    c.select(2, &LOGS, "b:planks", same);
    // Other ingredients with another conflict: the first match there.
    let sticks = [Some("planks"), None, Some("planks"), None];
    assert_eq!(c.choose(2, grid(&sticks), ids(&["a:stick", "c:stick"]), vec!["s", "t"], same), 0);
    // Back to the logs, in a crafting table this time (same recipes): still the pick.
    let table = [Some("log"), None, None, None, None, None, None, None, None];
    assert_eq!(c.choose(3, grid(&table), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same), 1);
    // The same recipes found in another order keep the picked recipe, not its place.
    assert_eq!(c.choose(2, grid(&LOGS), ids(&["b:planks", "a:planks"]), vec!["b", "a"], same), 0);
}

#[test]
fn the_last_pick_counts_in_a_new_set_that_has_it() {
    let mut c: Choices<S> = Choices::default();
    c.choose(2, grid(&LOGS), ids(&["a:x", "b:x"]), vec!["a", "b"], same);
    c.select(2, &LOGS, "b:x", same);
    let other = [Some("log"), Some("log"), None, None];
    // Polymorph's one selected recipe: made where it matches.
    assert_eq!(c.choose(2, grid(&other), ids(&["a:x", "c:x", "b:x"]), vec!["a", "c", "b"], same), 2);
}

#[test]
fn a_viewer_transfer_picks_its_recipe() {
    let mut c: Choices<S> = Choices::default();
    c.prefer("b:planks");
    assert_eq!(c.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same), 1);
    // Remembered for that set afterwards.
    assert_eq!(c.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same), 1);
    // A transfer into a grid without that conflict waits for one that has it.
    c.prefer("c:stick");
    let sticks = [Some("planks"), None, Some("planks"), None];
    assert_eq!(c.choose(2, grid(&sticks), ids(&["a:stick", "c:stick"]), vec!["s", "t"], same), 1);
}

#[test]
fn the_selector_finds_its_grid_and_only_its_grid() {
    let mut c: Choices<S> = Choices::default();
    c.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same);
    assert_eq!(c.conflict(2, &LOGS, same).map(|k| k.ids.len()), Some(2));
    assert!(c.conflict(2, &[Some("dirt"), None, None, None], same).is_none());
    assert!(c.conflict(3, &LOGS, same).is_none(), "another size is another grid");
    assert!(!c.select(2, &[Some("dirt"), None, None, None], "a:planks", same));
    assert!(!c.select(2, &LOGS, "z:none", same), "a recipe outside the conflict");
    // Recording the same grid again keeps one entry; the list is bounded.
    c.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same);
    for i in 0..RECENT + 4 {
        let item: S = Box::leak(format!("item{i}").into_boxed_str());
        c.choose(2, vec![Some(item), None, None, None], ids(&["a:p", "b:p"]), vec!["a", "b"], same);
    }
    assert!(c.conflict(2, &LOGS, same).is_none(), "the oldest grid is forgotten");
}

#[test]
fn ids_are_qualified() {
    assert_eq!(qualified("oak_planks"), "minecraft:oak_planks");
    assert_eq!(qualified("ae2:silicon"), "ae2:silicon");
    assert!(MAX_SHOWN == 15);
}

#[test]
fn polymorphs_layout() {
    // A crafting table's result slot item at (124, 35) in a panel at (0, 0).
    let b = layout::button([124.0, 35.0, 16.0, 16.0]);
    assert_eq!(b, [124.0, 13.0, 16.0, 16.0]);
    // One result: centred on the button's x - 4.
    assert_eq!(layout::outputs(b, 1), vec![[120.0, -13.0, 25.0, 25.0]]);
    // Two: -25 * 1 + 13 = -12 from there.
    let two = layout::outputs(b, 2);
    assert_eq!(two[0][0], 108.0);
    assert_eq!(two[1][0], 133.0);
    // Three: -25 from there, 25 apart.
    let three = layout::outputs(b, 3);
    assert_eq!(three.iter().map(|r| r[0]).collect::<Vec<_>>(), vec![95.0, 120.0, 145.0]);
    assert!(layout::inside(b, [124.0, 13.0]));
    assert!(!layout::inside(b, [140.0, 13.0]));
}

#[test]
fn the_server_makes_the_pick_it_hears() {
    // The server's own record of the grid, as its matcher saw it.
    let mut server: Choices<S> = Choices::default();
    assert_eq!(server.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same), 0);
    // The client's pick names the recipes, not the grid.
    let pick = Pick { id: "b:planks".into(), set: ids(&["a:planks", "b:planks"]) };
    let heard = Pick::decode(&pick.encode()).expect("a pick survives the trip");
    assert_eq!(heard, pick);
    assert!(server.pick(&heard.set, &heard.id));
    assert!(server.take_changed());
    assert_eq!(server.conflict(2, &LOGS, same).map(|c| c.chosen), Some(1), "the selector shows the pick");
    assert_eq!(server.choose(2, grid(&LOGS), ids(&["a:planks", "b:planks"]), vec!["a", "b"], same), 1);
    // A recipe outside the set is refused, and so is a message that names none.
    assert!(!server.pick(&heard.set, "c:planks"));
    assert_eq!(Pick::decode(b"b:planks\nb:planks"), None);
    assert_eq!(Pick::decode(b"c:x\na:x\nb:x"), None);
}
