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
}

#[derive(Default)]
struct LinkHover(Option<Entry>);

impl Global for LinkHover {}

/// The link under the pointer in `window`, if there is one.
pub fn hovered_link(window: &Window, cx: &App) -> Option<HoveredLink> {
    let entry = cx.try_global::<LinkHover>()?.0.as_ref()?;
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
    match link {
        Some(link) => {
            let same = slot.0.as_ref().is_some_and(|entry| {
                entry.owner == owner && entry.window == window && entry.link.url == link.url
            });
            if same {
                return false;
            }
            slot.0 = Some(Entry {
                owner,
                window,
                link: HoveredLink {
                    url: link.url.clone(),
                    title: link.title.clone(),
                    position,
                },
            });
            true
        }
        None => {
            if slot.0.as_ref().is_some_and(|entry| entry.owner == owner) {
                slot.0 = None;
                true
            } else {
                false
            }
        }
    }
}
