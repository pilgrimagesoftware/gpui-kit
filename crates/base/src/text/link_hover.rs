//! The link under the pointer, published so an app can show where a link
//! goes before it is followed - a tooltip, or a browser-style status line.
//!
//! Each rendered paragraph knows which of its links the pointer is over (it
//! already switches to the link cursor), but nothing outside it could ask.
//! A paragraph now records that link here: when the pointer moves onto it,
//! and again at paint, so a link scrolled or streamed under a still pointer
//! is found too. The slot remembers which paragraph wrote it, and only that
//! paragraph clears it, so one paragraph losing the pointer does not erase
//! the link another has just reported.
//!
//! Reading is [`hovered_link`]. A change redraws the window, so a reader in
//! that window's render sees it the same frame or the next.
//!
//! A report also lapses when the paragraph that made it stops being painted
//! with the pointer still on it - the view switched by keyboard, the row
//! virtualized away, the text replaced - since then nothing it owns runs to
//! clear it. Each frame of a window (see [`begin_frame`], from the window's
//! `TextSelectionLayer`) gets an epoch; the owner's paint re-stamps its
//! report with it, which changes nothing a reader sees and redraws nothing;
//! and after any frame that ends with a report its owner did not re-stamp,
//! the report is dropped and the window drawn once more.

use std::collections::HashMap;

use gpui::{App, Global, Pixels, Point, SharedString, Window, WindowId};

use crate::text::node::LinkMark;

/// A link the pointer is resting on.
#[derive(Clone, Debug, PartialEq)]
pub struct HoveredLink {
    /// Where the link goes.
    pub url: SharedString,
    /// The Markdown link title, `[text](url "title")`, if it has one.
    pub title: Option<SharedString>,
    /// The pointer's position when the link was entered, in window
    /// coordinates.
    pub position: Point<Pixels>,
}

struct Entry {
    /// The paragraph that reported it: its inline state's address, which is
    /// stable for as long as the paragraph is on screen.
    owner: usize,
    window: WindowId,
    link: HoveredLink,
    /// The window's epoch when the owner last reported it.
    confirmed: u64,
}

#[derive(Default)]
struct LinkHover {
    entry: Option<Entry>,
    /// Each window's frame epoch.
    epochs: HashMap<WindowId, u64>,
}

impl Global for LinkHover {}

/// The link under the pointer in `window`, if there is one.
pub fn hovered_link(window: &Window, cx: &App) -> Option<HoveredLink> {
    let entry = cx.try_global::<LinkHover>()?.entry.as_ref()?;
    (entry.window == window.window_handle().window_id()).then(|| entry.link.clone())
}

/// Records what `owner` has under the pointer: `link`, or nothing. Nothing
/// clears the slot only if `owner` is the one that filled it. Reports
/// whether the slot changed, so the caller can redraw.
pub(crate) fn publish(
    owner: usize,
    link: Option<&LinkMark>,
    position: Point<Pixels>,
    window: &Window,
    cx: &mut App,
) -> bool {
    let window = window.window_handle().window_id();
    let slot = cx.default_global::<LinkHover>();
    let epoch = slot.epochs.get(&window).copied().unwrap_or_default();
    match link {
        Some(link) => {
            if let Some(entry) = slot.entry.as_mut().filter(|entry| {
                entry.owner == owner && entry.window == window && entry.link.url == link.url
            }) {
                // Still there: re-stamped, which no reader sees.
                entry.confirmed = epoch;
                return false;
            }
            slot.entry = Some(Entry {
                owner,
                window,
                link: HoveredLink {
                    url: link.url.clone(),
                    title: link.title.clone(),
                    position,
                },
                confirmed: epoch,
            });
            true
        }
        None => {
            if slot
                .entry
                .as_ref()
                .is_some_and(|entry| entry.owner == owner)
            {
                slot.entry = None;
                true
            } else {
                false
            }
        }
    }
}

/// Starts a frame of `window`: a new epoch, and - when the window has a
/// report - a check after the frame that its owner re-stamped it, which
/// drops it and redraws the window if not. Called from the window's
/// `TextSelectionLayer`, which every `Root` draws first, every frame.
pub(crate) fn begin_frame(window: &Window, cx: &mut App) {
    let id = window.window_handle().window_id();
    let hover = cx.default_global::<LinkHover>();
    let epoch = {
        let epoch = hover.epochs.entry(id).or_default();
        *epoch += 1;
        *epoch
    };
    if !hover.entry.as_ref().is_some_and(|entry| entry.window == id) {
        return;
    }
    let handle = window.window_handle();
    cx.defer(move |cx| {
        let hover = cx.default_global::<LinkHover>();
        let lapsed = hover
            .entry
            .as_ref()
            .is_some_and(|entry| entry.window == id && entry.confirmed < epoch);
        if lapsed {
            hover.entry = None;
            let _ = handle.update(cx, |_, window, _| window.refresh());
        }
    });
}
