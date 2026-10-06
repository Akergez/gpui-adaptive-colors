//! Which colour the themes are made from, and keeping them made from it.
//!
//! [`AdaptiveColors`] is a GPUI global: the choice between the system's
//! colour and a named one, what the system last said, and the two schemes
//! that follow. Every change to any of those goes through here and ends in
//! the same place — both themes rebuilt and the one showing reapplied.
//!
//! Asking the system blocks (on Linux it is a round trip over D-Bus) and so
//! never happens on the main thread. A thread of its own waits on the system
//! for as long as the application runs and sends each answer over a channel
//! to a task on the main thread, which is the only thing that touches the
//! global. That leaves the first frame without an answer; an application
//! that kept the last one hands it back through [`Options::remembered`] and
//! starts in the right colours.

use std::ops::ControlFlow;
use std::rc::Rc;
use std::thread;

use adaptive_colors::{Color, Scheme, Seed, Variant, system};
use futures::StreamExt as _;
use futures::channel::mpsc::{self, UnboundedSender};
use gpui_kit::component::Theme;
use gpui_kit::{App, Global};

use crate::mapping::theme_config;
use crate::roles::SchemeColors;

/// Where the colour comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Whatever the system says, as it changes; the fallback where it says
    /// nothing.
    System,
    /// This one, whatever the system says.
    Seed(Seed),
}

impl From<Color> for Source {
    fn from(color: Color) -> Self {
        Source::Seed(Seed::Color(color))
    }
}

/// How [`init`] should set things up.
#[derive(Debug, Clone)]
pub struct Options {
    source: Source,
    fallback: Color,
    remembered: Option<Seed>,
    variant: Variant,
    contrast: f64,
}

impl Options {
    /// Follows the system, with `fallback` as the colour where the system
    /// has none to give: an older Android, a desktop without an accent
    /// colour, a platform this crate cannot ask yet.
    pub fn new(fallback: Color) -> Self {
        Options {
            source: Source::System,
            fallback,
            remembered: None,
            variant: Variant::default(),
            contrast: 0.0,
        }
    }

    pub fn source(mut self, source: impl Into<Source>) -> Self {
        self.source = source.into();
        self
    }

    /// What the system said the last time the application ran, to draw with
    /// until it has been asked again. See [`AdaptiveColors::system_seed`].
    pub fn remembered(mut self, seed: Option<Seed>) -> Self {
        self.remembered = seed;
        self
    }

    /// How palettes follow from a single colour. It does not apply to a
    /// system that hands over finished palettes, as Android does.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// From -1 to 1; 0 is standard and 1 is high contrast.
    pub fn contrast(mut self, contrast: f64) -> Self {
        self.contrast = contrast;
        self
    }
}

pub struct AdaptiveColors {
    source: Source,
    fallback: Color,
    variant: Variant,
    contrast: f64,
    /// What the system last said, or what was remembered of it until it
    /// says something.
    system: Option<Seed>,
    light: SchemeColors,
    dark: SchemeColors,
    /// The way answers reach the main thread, kept for [`refresh`].
    answers: UnboundedSender<Option<Seed>>,
}

impl Global for AdaptiveColors {}

impl AdaptiveColors {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn source(&self) -> Source {
        self.source
    }

    /// What the system last said its colour was. An application that wants
    /// its first frame in the right colours observes this global, keeps the
    /// value (a [`Seed`] writes itself as a string), and hands it back to
    /// [`Options::remembered`] on its next run.
    pub fn system_seed(&self) -> Option<Seed> {
        self.system
    }

    /// The seed the themes are made from right now.
    pub fn seed(&self) -> Seed {
        match self.source {
            Source::Seed(seed) => seed,
            Source::System => self.system.unwrap_or(Seed::Color(self.fallback)),
        }
    }

    pub(crate) fn colors(&self, dark: bool) -> &SchemeColors {
        if dark { &self.dark } else { &self.light }
    }

    fn schemes(&self) -> [Scheme; 2] {
        [false, true].map(|dark| Scheme::new(&self.seed(), self.variant, self.contrast, dark))
    }
}

/// Makes the toolkit's themes from a scheme and keeps them that way. Call it
/// once, after `gpui_kit::init`.
pub fn init(options: Options, cx: &mut App) {
    let (answers, mut incoming) = mpsc::unbounded::<Option<Seed>>();
    let mut colors = AdaptiveColors {
        source: options.source,
        fallback: options.fallback,
        variant: options.variant,
        contrast: options.contrast,
        system: options.remembered,
        // Replaced two lines down; there is no scheme to convert before
        // there is something to ask for one.
        light: SchemeColors::from(&Scheme::light(&options.fallback.into())),
        dark: SchemeColors::from(&Scheme::dark(&options.fallback.into())),
        answers: answers.clone(),
    };
    let schemes = colors.schemes();
    [colors.light, colors.dark] = schemes.each_ref().map(SchemeColors::from);
    cx.set_global(colors);
    install(&schemes, cx);

    cx.spawn(async move |cx| {
        while let Some(seed) = incoming.next().await {
            cx.update(|cx| system_answered(seed, cx));
        }
    })
    .detach();
    // If the thread cannot be started there is nobody to ask, which is the
    // same as a system with nothing to say.
    let _ = thread::Builder::new()
        .name("adaptive-colors".into())
        .spawn(move || {
            system::watch(|seed| match answers.unbounded_send(seed) {
                Ok(()) => ControlFlow::Continue(()),
                Err(_) => ControlFlow::Break(()),
            })
        });
}

/// Changes where the colour comes from.
pub fn set_source(source: impl Into<Source>, cx: &mut App) {
    let source = source.into();
    if AdaptiveColors::global(cx).source != source {
        change(cx, |colors| colors.source = source);
    }
}

/// Asks the system again. Only needed where the system does not announce a
/// change — Android, where the moment to ask is when the application comes
/// back to the front. Elsewhere it does no harm.
pub fn refresh(cx: &App) {
    let answers = AdaptiveColors::global(cx).answers.clone();
    let _ = thread::Builder::new()
        .name("adaptive-colors".into())
        .spawn(move || {
            let _ = answers.unbounded_send(system::read());
        });
}

fn system_answered(seed: Option<Seed>, cx: &mut App) {
    // The same answer again is nothing to tell an observer about.
    if AdaptiveColors::global(cx).system != seed {
        change(cx, |colors| colors.system = seed);
    }
}

/// Makes a change and brings the themes in step with it, if it changed the
/// seed they are made from: a new answer from the system changes nothing
/// while the application has named a colour of its own.
fn change(cx: &mut App, change: impl FnOnce(&mut AdaptiveColors)) {
    let colors = cx.global_mut::<AdaptiveColors>();
    let seed = colors.seed();
    change(colors);
    if colors.seed() == seed {
        return;
    }
    let schemes = colors.schemes();
    [colors.light, colors.dark] = schemes.each_ref().map(SchemeColors::from);
    install(&schemes, cx);
}

/// Puts a light and a dark scheme behind the theme's two modes and shows
/// again whichever mode is up.
fn install([light, dark]: &[Scheme; 2], cx: &mut App) {
    let theme = Theme::global_mut(cx);
    theme.light_theme = Rc::new(theme_config(light));
    theme.dark_theme = Rc::new(theme_config(dark));
    let mode = theme.mode;
    Theme::change(mode, None, cx);
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::{ActiveTheme as _, ThemeMode};
    use gpui_kit::{Hsla, TestAppContext};

    use super::*;
    use crate::ActiveScheme as _;
    use crate::roles::hsla;

    const VIOLET: Color = Color::from_u32(0x6750a4);
    const GREEN: Color = Color::from_u32(0x386a20);

    fn start(options: Options, cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            init(options, cx);
        });
    }

    fn primary_of(color: Color, dark: bool) -> Hsla {
        let scheme = Scheme::new(&color.into(), Variant::default(), 0.0, dark);
        hsla(scheme.primary())
    }

    #[gpui_kit::test]
    fn a_named_colour_is_what_the_theme_is_made_from(cx: &mut TestAppContext) {
        start(Options::new(GREEN).source(VIOLET), cx);
        cx.update(|cx| {
            let dark = cx.theme().mode.is_dark();
            assert_eq!(cx.theme().primary, primary_of(VIOLET, dark));
            assert_eq!(cx.scheme().primary, cx.theme().primary);
        });
    }

    #[gpui_kit::test]
    fn switching_between_light_and_dark_stays_in_the_scheme(cx: &mut TestAppContext) {
        start(Options::new(GREEN).source(VIOLET), cx);
        cx.update(|cx| {
            for (mode, dark) in [(ThemeMode::Dark, true), (ThemeMode::Light, false)] {
                Theme::change(mode, None, cx);
                assert_eq!(cx.theme().primary, primary_of(VIOLET, dark));
                assert_eq!(cx.scheme().primary, primary_of(VIOLET, dark));
            }
        });
    }

    #[gpui_kit::test]
    fn a_change_of_source_is_drawn_at_once(cx: &mut TestAppContext) {
        start(Options::new(GREEN).source(VIOLET), cx);
        cx.update(|cx| {
            let dark = cx.theme().mode.is_dark();
            set_source(GREEN, cx);
            assert_eq!(cx.theme().primary, primary_of(GREEN, dark));
            assert_eq!(AdaptiveColors::global(cx).seed(), Seed::Color(GREEN));
        });
    }

    #[gpui_kit::test]
    fn what_the_system_said_last_time_is_used_until_it_says_otherwise(cx: &mut TestAppContext) {
        start(
            Options::new(GREEN).remembered(Some(Seed::Color(VIOLET))),
            cx,
        );
        cx.update(|cx| {
            let dark = cx.theme().mode.is_dark();
            assert_eq!(cx.theme().primary, primary_of(VIOLET, dark));

            system_answered(None, cx);
            assert_eq!(cx.theme().primary, primary_of(GREEN, dark));
        });
    }

    #[gpui_kit::test]
    fn the_system_is_not_listened_to_while_a_colour_is_named(cx: &mut TestAppContext) {
        start(Options::new(GREEN).source(VIOLET), cx);
        cx.update(|cx| {
            let dark = cx.theme().mode.is_dark();
            system_answered(Some(Seed::Color(GREEN)), cx);
            assert_eq!(cx.theme().primary, primary_of(VIOLET, dark));
            // It is still noted, for when the system is followed again.
            assert_eq!(
                AdaptiveColors::global(cx).system_seed(),
                Some(Seed::Color(GREEN))
            );

            set_source(Source::System, cx);
            assert_eq!(cx.theme().primary, primary_of(GREEN, dark));
        });
    }

    #[gpui_kit::test]
    fn sizes_the_application_chose_survive_a_change_of_colour(cx: &mut TestAppContext) {
        start(Options::new(GREEN).source(VIOLET), cx);
        cx.update(|cx| {
            let chosen = cx.theme().font_size * 1.25;
            Theme::global_mut(cx).font_size = chosen;
            set_source(GREEN, cx);
            assert_eq!(cx.theme().font_size, chosen);
        });
    }
}
