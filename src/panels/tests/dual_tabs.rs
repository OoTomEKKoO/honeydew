//! Two pages at once, each under a tab strip of its own (the GMSynth in a portrait phone window): what a pick on a
//! strip does, and that both pages are laid out one above the other.

use std::collections::HashMap;

use eframe::egui::{Rect, pos2};

use super::U;
use crate::panels::{LOWER_TABS, PanelState, panel_for, place};
use crate::song::ModuleInstance;

/// The GMSynth's card content in a portrait phone window: 17 U wide, 14 U high.
const RECT: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(17.0 * U, 14.0 * U),
};

fn rects(state: &PanelState) -> HashMap<u32, Rect> {
    let panel = panel_for(2).unwrap();
    let module = ModuleInstance::new(2, 1);
    place(panel, &module, state, RECT, U)
        .into_iter()
        .map(|(leaf, rect)| (leaf.id, rect))
        .collect()
}

fn dual(tab: usize, tab2: usize) -> PanelState {
    PanelState {
        dual_tabs: true,
        tab,
        tab2: Some(tab2),
        ..PanelState::default()
    }
}

#[test]
fn each_strip_picks_the_page_under_it() {
    let mut state = dual(0, 1);
    assert!(state.pick_tab(false, 3));
    assert_eq!((state.tab, state.tab2), (3, Some(1)));
    assert!(state.pick_tab(true, 5));
    assert_eq!((state.tab, state.tab2), (3, Some(5)));
    // Picking the page a strip already shows changes nothing.
    assert!(!state.pick_tab(true, 5));
    assert!(!state.pick_tab(false, 3));
}

#[test]
fn a_page_picked_on_the_other_strip_swaps_the_two() {
    let mut state = dual(2, 6);
    // The upper strip picks the lower page: they trade places.
    assert!(state.pick_tab(false, 6));
    assert_eq!((state.tab, state.tab2), (6, Some(2)));
    // The lower strip picks the upper page: again.
    assert!(state.pick_tab(true, 6));
    assert_eq!((state.tab, state.tab2), (2, Some(6)));
}

#[test]
fn both_pages_show_under_their_strips_in_slot_order() {
    // MOD (2) on top, LEVEL (0) below: the slots, not the tab order, give the order.
    let state = dual(2, 0);
    let r = rects(&state);
    let (upper_strip, lower_strip) = (r[&0], r[&LOWER_TABS]);
    let (top_page, bottom_page) = (r[&31000], r[&41000]);
    assert!(
        upper_strip.bottom() <= top_page.top(),
        "{upper_strip:?} {top_page:?}"
    );
    assert!(
        top_page.bottom() <= lower_strip.top(),
        "{top_page:?} {lower_strip:?}"
    );
    assert!(
        lower_strip.bottom() <= bottom_page.top(),
        "{lower_strip:?} {bottom_page:?}"
    );
    // Only those two pages are laid out.
    assert!(!r.contains_key(&21100), "the OSC page is hidden");
}

#[test]
fn one_page_and_one_strip_without_two_at_once() {
    let mut state = dual(1, 0);
    state.dual_tabs = false;
    let r = rects(&state);
    assert!(r.contains_key(&21100) && !r.contains_key(&41000));
    assert!(!r.contains_key(&LOWER_TABS), "no lower strip");
}
