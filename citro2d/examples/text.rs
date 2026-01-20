use citro2d::{
    drawable::RectangleSolid,
    font::Font,
    render::{Color, TargetExt as _},
    text::{HorizontalAlignment, Text, TextDrawStyle},
};
use ctru::{prelude::*, services::gfx::TopScreen3D};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gfx = Gfx::new().expect("Couldn't obtain GFX controller");
    let mut hid = Hid::new().expect("Couldn't obtain HID controller");
    let apt = Apt::new().expect("Couldn't obtain APT controller");

    ctru::set_panic_hook(false);

    let mut citro2d_instance = citro2d::Instance::new().expect("Couldn't obtain citro2d instance");
    let top_screen = TopScreen3D::from(&gfx.top_screen);
    let (top_left, _) = top_screen.split_mut();
    let mut top_target = citro2d_instance
        .create_screen_target(top_left)
        .expect("failed to create render target");

    let _bottom_screen = Console::new(gfx.bottom_screen.borrow_mut());

    let clr_clear = Color::new(255, 216, 176);

    let system_font = Font::get_shared();

    let mut hello_text = Text::new(
        (200., 32.).into(),
        TextDrawStyle::default()
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(citro2d::text::VerticalAlignment::Baseline),
    )?;
    hello_text.parse("hello, Citro2D!", &system_font)?;

    let mut wrapping_text = Text::new(
        (32., 64.).into(),
        TextDrawStyle::default().with_word_wrap(Some(400. - (32. * 2.))),
    )?;
    wrapping_text.parse("The quick brown fox jumps over the lazy dog.", &system_font)?;

    let mut scalar_delta = 0.025;

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_down().contains(KeyPad::START) {
            break;
        }

        hello_text.style.scale.0 += scalar_delta;
        if !(1.0..=2.0).contains(&hello_text.style.scale.0) {
            scalar_delta *= -1.
        }

        (top_target, ()) = citro2d_instance
            .render_to_target(top_target, |_instance, mut render_target| {
                render_target.clear_with_color(clr_clear);

                let (point, size) = hello_text.estimate_bounding_box(&system_font);
                render_target.render_drawable(&RectangleSolid {
                    point,
                    size,
                    color: Color::new(0xff, 0xff, 0xff),
                });

                render_target.render_drawable(&hello_text);
                render_target.render_drawable(&wrapping_text);

                (render_target, ())
            })
            .unwrap();
    }

    Ok(())
}
