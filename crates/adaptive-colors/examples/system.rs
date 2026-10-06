//! Prints the colour the system has chosen and a few roles of the scheme it
//! gives: `cargo run -p adaptive-colors --features system --example system`.

use adaptive_colors::{Scheme, system};

fn main() {
    let Some(seed) = system::read() else {
        println!("The system has no colour to give.");
        return;
    };
    println!("seed {seed}");
    for scheme in [Scheme::light(&seed), Scheme::dark(&seed)] {
        println!(
            "{}: primary {} on {}, surface {}, text {}",
            if scheme.is_dark() { "dark" } else { "light" },
            scheme.primary(),
            scheme.on_primary(),
            scheme.surface(),
            scheme.on_surface(),
        );
    }
}
