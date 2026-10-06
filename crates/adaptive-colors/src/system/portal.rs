//! The accent colour of a freedesktop desktop, from the settings portal.
//!
//! The portal is spoken to directly over D-Bus rather than through a portal
//! library: it is one method and one signal, and those libraries are
//! asynchronous and want a runtime chosen for them.

use std::ops::ControlFlow;

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedValue, Value};

use crate::{Color, Seed};

const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "accent-color";

fn settings() -> zbus::Result<Proxy<'static>> {
    Proxy::new(
        &Connection::session()?,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
    )
}

fn read_from(settings: &Proxy<'_>) -> Option<Seed> {
    // `ReadOne` is the current method; a portal from before it has only
    // `Read`, which wraps its answer in a second variant.
    let value: OwnedValue = settings
        .call("ReadOne", &(NAMESPACE, KEY))
        .or_else(|_| settings.call("Read", &(NAMESPACE, KEY)))
        .ok()?;
    accent(&value).map(Seed::Color)
}

/// The colour in a portal's answer: three reals from 0 to 1. Anything else —
/// and the portal documents out-of-range values as its way of saying "no
/// preference" — is no colour.
fn accent(value: &Value<'_>) -> Option<Color> {
    match value {
        Value::Value(inner) => accent(inner),
        Value::Structure(channels) => match channels.fields() {
            [Value::F64(red), Value::F64(green), Value::F64(blue)] => {
                let byte = |channel: f64| {
                    (0.0..=1.0)
                        .contains(&channel)
                        .then(|| (channel * 255.0).round() as u8)
                };
                Some(Color::rgb(byte(*red)?, byte(*green)?, byte(*blue)?))
            }
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn read() -> Option<Seed> {
    read_from(&settings().ok()?)
}

pub(super) fn watch(mut on_change: impl FnMut(Option<Seed>) -> ControlFlow<()>) {
    let Ok(settings) = settings() else {
        let _ = on_change(None);
        return;
    };
    // Subscribed before the first read, so a change between the two is not
    // lost.
    let changes = settings.receive_signal("SettingChanged");
    if on_change(read_from(&settings)).is_break() {
        return;
    }
    let Ok(changes) = changes else { return };
    for change in changes {
        let Ok((namespace, key, value)) = change
            .body()
            .deserialize::<(String, String, OwnedValue)>()
        else {
            continue;
        };
        if namespace == NAMESPACE && key == KEY && on_change(accent(&value).map(Seed::Color)).is_break() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use zbus::zvariant::Structure;

    use super::*;

    fn answer(red: f64, green: f64, blue: f64) -> Value<'static> {
        Value::Structure(Structure::from((red, green, blue)))
    }

    #[test]
    fn three_reals_are_a_colour() {
        assert_eq!(
            accent(&answer(1.0, 0.611765, 0.694118)),
            Some(Color::rgb(255, 156, 177))
        );
    }

    #[test]
    fn the_older_method_wraps_its_answer_once_more() {
        let wrapped = Value::Value(Box::new(answer(0.0, 0.0, 1.0)));
        assert_eq!(accent(&wrapped), Some(Color::rgb(0, 0, 255)));
    }

    #[test]
    fn out_of_range_is_the_portal_saying_no_preference() {
        assert_eq!(accent(&answer(-1.0, -1.0, -1.0)), None);
        assert_eq!(accent(&answer(0.2, 1.5, 0.2)), None);
        assert_eq!(accent(&Value::U32(7)), None);
    }
}
