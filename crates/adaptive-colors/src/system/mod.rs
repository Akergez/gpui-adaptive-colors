//! The colour the system has chosen.
//!
//! Every platform keeps it somewhere else and says it differently, so this
//! is one question with an answer per platform:
//!
//! - **Linux and FreeBSD**: the desktop's accent colour, from the settings
//!   portal (`org.freedesktop.appearance`, `accent-color`). One colour, and a
//!   signal when it changes.
//! - **Android 12 and later**: the palettes the system derived from the
//!   wallpaper, as five key colours. Nothing announces a change; ask again
//!   when the application comes back to the front.
//! - **Elsewhere**, and wherever the above is missing (an older Android, a
//!   desktop without the portal): no answer.
//!
//! Both functions block — the portal is a round trip over D-Bus — and
//! neither knows about any executor. Call them from a thread of your own.

use std::ops::ControlFlow;

use crate::Seed;

#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod portal;

/// What the system's colour is now, or `None` where it has none to give.
pub fn read() -> Option<Seed> {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return portal::read();
    #[cfg(target_os = "android")]
    return android::read();
    #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "android")))]
    None
}

/// Tells `on_change` what the system's colour is now, then again every time
/// it changes, for as long as `on_change` answers [`ControlFlow::Continue`].
///
/// It returns once it has nothing more to say: when told to stop, or straight
/// after the first answer on a platform that does not announce changes.
/// Stopping takes effect at the next change, since that is when there is an
/// answer to give.
pub fn watch(on_change: impl FnMut(Option<Seed>) -> ControlFlow<()>) {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    portal::watch(on_change);
    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    {
        let mut on_change = on_change;
        let _ = on_change(read());
    }
}
