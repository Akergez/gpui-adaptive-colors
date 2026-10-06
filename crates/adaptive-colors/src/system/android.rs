//! The palettes Android derives from the wallpaper.
//!
//! Since Android 12 they are ordinary colour resources of the system:
//! `system_accent1_500` and so on, five palettes of thirteen shades each.
//! They are read by name through JNI, with no Java of our own, and found
//! through `ndk-context`, which whatever started the native code (the
//! `android-activity` crate, for one) fills in with the virtual machine and
//! the activity.
//!
//! The shades are not the tones a Material 3 scheme needs — its surfaces sit
//! at tones the system never stored — so what is taken from each palette is
//! what defines it, its hue and chroma, as its most colourful shade. The
//! scheme is then worked out from those the usual way.

use jni::objects::{JObject, JValue};
use jni::{Env, JavaVM};

use crate::{Color, KeyColors, Seed};

/// In the order [`KeyColors::new`] takes them.
const PALETTES: [&str; 5] = ["accent1", "accent2", "accent3", "neutral1", "neutral2"];

/// The middle of a palette, where a shade is neither too light nor too dark
/// to show its chroma.
const SHADES: [u16; 5] = [300, 400, 500, 600, 700];

pub(super) fn read() -> Option<Seed> {
    // `ndk-context` panics when nothing has filled it in, which is a reason
    // to have no answer and not one to take the application down.
    let context = std::panic::catch_unwind(ndk_context::android_context).ok()?;
    if context.vm().is_null() || context.context().is_null() {
        return None;
    }
    // SAFETY: `ndk-context` hands out the process's one virtual machine and a
    // global reference to a `Context`, both valid for as long as the process.
    let vm = unsafe { JavaVM::from_raw(context.vm().cast()) };
    vm.attach_current_thread(|env| -> jni::errors::Result<Option<Seed>> {
        // SAFETY: as above; the reference is global, so wrapping it does not
        // hand its ownership to this frame.
        let context = unsafe { JObject::from_raw(env, context.context().cast()) };
        key_colors(env, &context)
    })
    .ok()
    .flatten()
}

fn key_colors(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<Option<Seed>> {
    let resources = env
        .call_method(
            context,
            jni::jni_str!("getResources"),
            jni::jni_sig!("()Landroid/content/res/Resources;"),
            &[],
        )?
        .l()?;
    let mut keys = [Color::rgb(0, 0, 0); 5];
    for (key, palette) in keys.iter_mut().zip(PALETTES) {
        let mut shades = Vec::with_capacity(SHADES.len());
        for shade in SHADES {
            match color(env, &resources, &format!("system_{palette}_{shade}"))? {
                Some(color) => shades.push(color),
                // Before Android 12 there are no such resources.
                None => return Ok(None),
            }
        }
        *key = shades
            .into_iter()
            .max_by(|a, b| a.chroma().total_cmp(&b.chroma()))
            .expect("there is more than one shade");
    }
    let [primary, secondary, tertiary, neutral, neutral_variant] = keys;
    Ok(Some(Seed::KeyColors(KeyColors::new(
        primary,
        secondary,
        tertiary,
        neutral,
        neutral_variant,
    ))))
}

/// A colour resource of the system by name, or `None` if it has none by
/// that name.
fn color(
    env: &mut Env<'_>,
    resources: &JObject<'_>,
    name: &str,
) -> jni::errors::Result<Option<Color>> {
    // Its own frame, so that the strings of one lookup are gone before the
    // next: a thread has a limited number of local references.
    env.with_local_frame(8, |env| -> jni::errors::Result<Option<Color>> {
        let (name, kind, package) = (
            env.new_string(name)?,
            env.new_string("color")?,
            env.new_string("android")?,
        );
        let id = env
            .call_method(
                resources,
                jni::jni_str!("getIdentifier"),
                jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I"),
                &[
                    JValue::Object(&name),
                    JValue::Object(&kind),
                    JValue::Object(&package),
                ],
            )?
            .i()?;
        if id == 0 {
            return Ok(None);
        }
        let argb = env
            .call_method(
                resources,
                jni::jni_str!("getColor"),
                jni::jni_sig!("(ILandroid/content/res/Resources$Theme;)I"),
                &[JValue::Int(id), JValue::Object(&JObject::null())],
            )?
            .i()?;
        Ok(Some(Color::from_u32(argb as u32)))
    })
}
