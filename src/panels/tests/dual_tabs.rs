//! Two pages at once on a tab strip (the GMSynth in a portrait phone window): which pages a tap leaves shown, and
//! that both pages are laid out, one above the other.

use eframe::egui::{Rect, pos2};

use super::U;
use crate::panels::{PanelState, panel_for, place};
use crate::song::ModuleInstance;

/// The GMSynth's card content in a portrait phone window: 17 U wide, 13 U high.
const RECT: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(17.0 * U, 13.0 * U),
};

fn shown(state: &PanelState) -> Vec<u32> {
    let panel = panel_for(2).unwrap();
    let module = ModuleInstance::new(2, 1);
    let leaves = place(panel, &module, state, RECT, U);
    leaves.iter().map(|(leaf, _)| leaf.id).collect()
}

#[test]
fn a_tap_adds_a_page_and_drops_the_one_before_the_last() {
    let mut state = PanelState {
        dual_tabs: true,
        tab: 0,
        tab2: Some(1),
        ..PanelState::default()
    };
    // Page 3 joins page 0 as the newest; page 1, the oldest, drops.
    assert!(state.tap_tab(3));
    assert_eq!((state.tab, state.tab2), (3, Some(0)));
}

#[test]
fn a_tap_on_a_shown_page_hides_it_but_never_the_last_page() {
    let mut state = PanelState {
        dual_tabs: true,
        tab: 2,
        tab2: Some(5),
        ..PanelState::default()
    };
    // The other page stays, whichever of the two was tapped.
    assert!(state.tap_tab(5));
    assert_eq!((state.tab, state.tab2), (2, None));
    // The last page cannot be hidden.
    assert!(!state.tap_tab(2));
    assert_eq!((state.tab, state.tab2), (2, None));
    // A page tapped while one is shown joins it.
    assert!(state.tap_tab(4));
    assert_eq!((state.tab, state.tab2), (4, Some(2)));
    // The newest page leaving keeps the older one.
    assert!(state.tap_tab(4));
    assert_eq!((state.tab, state.tab2), (2, None));
}

#[test]
fn both_pages_are_shown_one_above_the_other_in_tab_order() {
    let mut state = PanelState {
        dual_tabs: true,
        tab: 1,
        tab2: Some(0),
        ..PanelState::default()
    };
    let panel = panel_for(2).unwrap();
    let module = ModuleInstance::new(2, 1);
    let rects: std::collections::HashMap<u32, Rect> = place(panel, &module, &state, RECT, U)
        .into_iter()
        .map(|(leaf, rect)| (leaf.id, rect))
        .collect();
    // FILTER (CUTOFF 11000) is page 0, OSC (A, 21100) page 1: FILTER sits on top, OSC below, neither overlapping.
    let (filter, osc) = (rects[&11000], rects[&21100]);
    assert!(filter.bottom() <= osc.top(), "{filter:?} {osc:?}");
    // The tab strip is above both, and only those two pages are laid out.
    assert!(rects[&0].bottom() <= filter.top());
    assert!(!rects.contains_key(&31000), "the MOD page is hidden");
    // Without two pages at once only the selected page shows.
    state.dual_tabs = false;
    let ids = shown(&state);
    assert!(ids.contains(&21100) && !ids.contains(&11000));
}
