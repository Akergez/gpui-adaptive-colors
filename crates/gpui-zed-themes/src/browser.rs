//! The registry as something to look through: a search box over a list of
//! extensions, each with a button that installs or removes it.
//!
//! The list is asked for once ([`Browser::load`]) and searched locally after
//! that, so typing costs no requests. It is not virtual and several hundred
//! entries long, so only the first few dozen of what matches are drawn; the
//! search box is the way to the rest.
//!
//! The view changes the [`Store`] and says so ([`BrowserEvent::Changed`]);
//! it does not know which theme the application wears or how it chooses one.
//! The requests and the disk work run on GPUI's background executor.
//!
//! An install goes on to its end if the view is dropped while it runs, but
//! then nobody is told: an application that can close the browser mid-way
//! reads the store again when it next shows its themes.

use std::collections::{BTreeSet, HashSet};

use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, AppContext, Context, ElementId, Entity, EventEmitter, SharedString, Subscription,
    Window, div,
};

use crate::registry::{Extension, Registry};
use crate::store::Store;

/// How many registry entries are drawn at once.
const SHOWN: usize = 40;

/// What is known of the registry's list.
enum Listing {
    NotAsked,
    Loading,
    Failed(String),
    Loaded(Vec<Extension>),
}

/// What a [`Browser`] tells whoever holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserEvent {
    /// An extension was installed or removed: the store holds other themes
    /// than it did.
    Changed,
}

pub struct Browser {
    registry: Registry,
    store: Store,
    search: Entity<InputState>,
    listing: Listing,
    installed: BTreeSet<String>,
    /// Extensions being installed or removed right now.
    busy: HashSet<String>,
    /// What the last install or removal had to say.
    notice: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<BrowserEvent> for Browser {}

impl Browser {
    pub fn new(
        registry: Registry,
        store: Store,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search Zed themes"));
        let typed = cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Browser {
            installed: store.installed(),
            registry,
            store,
            search,
            listing: Listing::NotAsked,
            busy: HashSet::new(),
            notice: None,
            _subscriptions: vec![typed],
        }
    }

    /// Asks the registry for its list, unless it is here or on its way; a
    /// list that failed to come is asked for again. Call it when the browser
    /// comes into view.
    pub fn load(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.listing, Listing::NotAsked | Listing::Failed(_)) {
            return;
        }
        self.listing = Listing::Loading;
        cx.notify();
        let registry = self.registry.clone();
        let listed =
            cx.background_spawn(async move { registry.list().map_err(|error| error.to_string()) });
        cx.spawn(async move |this, cx| {
            let listed = listed.await;
            let _ = this.update(cx, |this, cx| {
                this.listing = match listed {
                    Ok(extensions) => Listing::Loaded(extensions),
                    Err(error) => Listing::Failed(error),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Puts the keyboard in the search box.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.search
            .update(cx, |search, cx| search.focus(window, cx));
    }

    fn install(&mut self, extension: &Extension, cx: &mut Context<Self>) {
        let (registry, store) = (self.registry.clone(), self.store.clone());
        let (id, title) = (extension.id().to_string(), extension.title().to_string());
        let work = {
            let id = id.clone();
            move || {
                let files = registry.download(&id).map_err(|error| error.to_string())?;
                store.install(&id, &files).map_err(|error| error.to_string())
            }
        };
        self.change(id, work, cx, move |added| match added {
            1 => format!("{title}: one theme added."),
            added => format!("{title}: {added} themes added."),
        });
    }

    fn remove(&mut self, extension: &Extension, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let (id, title) = (extension.id().to_string(), extension.title().to_string());
        let work = {
            let id = id.clone();
            move || {
                store
                    .remove(&id)
                    .map(|()| 0)
                    .map_err(|error| error.to_string())
            }
        };
        self.change(id, work, cx, move |_| format!("{title} was removed."));
    }

    /// Runs an install or a removal off the main thread and then makes the
    /// list agree with the directory, whichever way it went.
    fn change(
        &mut self,
        id: String,
        work: impl FnOnce() -> Result<usize, String> + Send + 'static,
        cx: &mut Context<Self>,
        done: impl FnOnce(usize) -> String + 'static,
    ) {
        self.busy.insert(id.clone());
        self.notice = None;
        cx.notify();
        let store = self.store.clone();
        let worked = cx.background_spawn(async move {
            let result = work();
            (result, store.installed())
        });
        cx.spawn(async move |this, cx| {
            let (result, installed) = worked.await;
            let _ = this.update(cx, |this, cx| {
                this.busy.remove(&id);
                this.installed = installed;
                this.notice = Some(match result {
                    Ok(count) => done(count).into(),
                    Err(error) => error.into(),
                });
                // Also after a failure: a download that broke off half-way
                // has already replaced what was installed before.
                cx.emit(BrowserEvent::Changed);
                cx.notify();
            });
        })
        .detach();
    }

    fn rows(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let note = |text: SharedString| {
            div()
                .p_3()
                .text_sm()
                .text_color(muted)
                .child(text)
                .into_any_element()
        };
        let extensions = match &self.listing {
            Listing::NotAsked | Listing::Loading => {
                return note("Asking Zed's registry\u{2026}".into());
            }
            Listing::Failed(error) => {
                return note(format!("The registry did not answer: {error}").into());
            }
            Listing::Loaded(extensions) => extensions,
        };
        let query = self.search.read(cx).value();
        let (shown, total) = found(extensions, &query);
        if shown.is_empty() {
            return note("No theme extension matches that.".into());
        }

        let mut rows = v_flex().gap_1();
        for extension in &shown {
            let id = extension.id();
            let busy = self.busy.contains(id);
            let mut by = extension.author().map(str::to_string).unwrap_or_default();
            if !by.is_empty() {
                by.push_str(" \u{b7} ");
            }
            by.push_str(&format!(
                "{} downloads",
                downloads(extension.download_count())
            ));

            let button = Button::new(ElementId::Name(format!("zed-theme-{id}").into()))
                .small()
                .outline()
                .disabled(busy);
            let subject = (*extension).clone();
            let button = if self.installed.contains(id) {
                button
                    .label(if busy { "Removing\u{2026}" } else { "Remove" })
                    .on_click(cx.listener(move |this, _, _, cx| this.remove(&subject, cx)))
            } else {
                button
                    .label(if busy { "Installing\u{2026}" } else { "Install" })
                    .on_click(cx.listener(move |this, _, _, cx| this.install(&subject, cx)))
            };
            rows = rows.child(
                h_flex()
                    .gap_3()
                    .px_2()
                    .py_1p5()
                    .items_center()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(div().truncate().child(extension.title().to_string()))
                            .children(extension.description().map(|description| {
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .truncate()
                                    .child(description.to_string())
                            }))
                            .child(div().text_xs().text_color(muted).truncate().child(by)),
                    )
                    .child(button),
            );
        }
        if total > shown.len() {
            rows = rows.child(note(
                format!("{} of {total} shown. Search to find the rest.", shown.len()).into(),
            ));
        }
        rows.into_any_element()
    }
}

impl Render for Browser {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (muted, border, radius) = (
            cx.theme().muted_foreground,
            cx.theme().border,
            cx.theme().radius,
        );
        v_flex()
            .gap_2()
            .child(Input::new(&self.search).cleanable(true))
            // Above the list, not under it: the list is tall, and what an
            // install had to say should not need scrolling to.
            .children(
                self.notice
                    .clone()
                    .map(|notice| div().text_xs().text_color(muted).child(notice)),
            )
            .child(
                div()
                    .id("zed-themes")
                    .h_56()
                    .border_1()
                    .border_color(border)
                    .rounded(radius)
                    .overflow_y_scroll()
                    .child(self.rows(cx)),
            )
    }
}

/// The extensions a search leaves, in the registry's order, and how many
/// there were before the list was cut to what is drawn.
fn found<'a>(extensions: &'a [Extension], query: &str) -> (Vec<&'a Extension>, usize) {
    let needle = query.trim().to_lowercase();
    let matching: Vec<&Extension> = extensions
        .iter()
        .filter(|extension| extension.matches(&needle))
        .collect();
    let total = matching.len();
    (matching.into_iter().take(SHOWN).collect(), total)
}

/// A download count the way a list row has room for it.
fn downloads(count: u64) -> String {
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => format!("{}K", count / 1_000),
        _ => format!("{:.1}M", count as f64 / 1_000_000.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extension(id: &str, name: &str) -> Extension {
        serde_json::from_value(serde_json::json!({ "id": id, "name": name })).unwrap()
    }

    #[test]
    fn a_search_keeps_the_registry_order_and_ignores_case_and_edges() {
        let extensions = [
            extension("catppuccin", "Catppuccin"),
            extension("tokyo-night", "Tokyo Night"),
            extension("catppuccin-blur", "Catppuccin Blur"),
        ];
        let (shown, total) = found(&extensions, "  CATP ");
        assert_eq!(total, 2);
        let ids: Vec<_> = shown.iter().map(|extension| extension.id()).collect();
        assert_eq!(ids, ["catppuccin", "catppuccin-blur"]);
        assert_eq!(found(&extensions, "").1, 3);
    }

    #[test]
    fn a_long_list_is_cut_and_says_how_long_it_was() {
        let extensions: Vec<Extension> = (0..SHOWN + 25)
            .map(|index| extension(&format!("theme-{index}"), "Theme"))
            .collect();
        let (shown, total) = found(&extensions, "theme");
        assert_eq!(shown.len(), SHOWN);
        assert_eq!(total, SHOWN + 25);
    }

    #[test]
    fn a_download_count_is_short_enough_for_a_row() {
        assert_eq!(downloads(0), "0");
        assert_eq!(downloads(812), "812");
        assert_eq!(downloads(1_000), "1K");
        assert_eq!(downloads(496_067), "496K");
        assert_eq!(downloads(1_163_124), "1.2M");
    }
}
