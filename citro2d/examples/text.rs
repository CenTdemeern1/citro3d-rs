use citro2d::{
    drawable::{Line, RectangleSolid},
    font::Font,
    render::{Color, TargetExt as _},
    text::{HorizontalAlignment, Text, TextDrawStyle},
};
use ctru::{prelude::*, services::gfx::TopScreen3D};

fn main() {
    let gfx = Gfx::new().expect("Couldn't obtain GFX controller");
    let mut hid = Hid::new().expect("Couldn't obtain HID controller");
    let apt = Apt::new().expect("Couldn't obtain APT controller");
    let _romfs = ctru::services::romfs::RomFS::new().unwrap();

    ctru::set_panic_hook(false);

    let mut citro2d_instance = citro2d::Instance::new().expect("Couldn't obtain citro2d instance");
    let top_screen = TopScreen3D::from(&gfx.top_screen);
    let (top_left, _) = top_screen.split_mut();
    let mut top_target = citro2d_instance
        .create_screen_target(top_left)
        .expect("failed to create render target");

    let _bottom_screen = Console::new(gfx.bottom_screen.borrow_mut());

    let clr_clear = Color::new(255, 216, 176);

    let font = Font::get_shared();
    let style = TextDrawStyle::default().with_horizontal_alignment(HorizontalAlignment::Center);
    let mut hello_text = Text::new((200., 32.).into(), style);
    hello_text.parse(c"hello Citro2D!", &font);

    let font = unsafe { Font::from_file_path_unchecked("romfs:/papyrus.bcfnt").unwrap() };
    let style = TextDrawStyle::default()
        .with_word_wrap(200.)
        .with_color(Color::new(0, 0, 0));
    let mut papyrus_text = Text::new((32., 72.).into(), style);
    papyrus_text.parse(
        c"The quick brown fox jumps over the lazy dog. Look, this text has word wrapping!",
        &font,
    );

    let mut scalar_delta = 0.025;

    while apt.main_loop() {
        hid.scan_input();

        hello_text.style.scalars.0 += scalar_delta;
        if !(1.0..=2.0).contains(&hello_text.style.scalars.0) {
            scalar_delta *= -1.
        }

        (top_target, ()) = citro2d_instance
            .render_to_target(top_target, |_instance, mut render_target| {
                render_target.clear_with_color(clr_clear);

                let (point, size) = hello_text.estimate_bounding();
                render_target.render_drawable(&RectangleSolid {
                    point,
                    size,
                    color: Color::new(0xff, 0xff, 0xff),
                });

                let x = papyrus_text.point.x + papyrus_text.style.word_wrap.unwrap();
                let y = papyrus_text.point.y;
                render_target.render_drawable(&Line {
                    start: (x, y).into(),
                    end: (x, y + 300.).into(),
                    start_color: Color::new(0, 0, 0),
                    end_color: Color::new(255, 255, 255),
                    depth: 0.,
                    thickness: 2.,
                });

                render_target.render_drawable(&hello_text);
                render_target.render_drawable(&papyrus_text);

                (render_target, ())
            })
            .unwrap();
    }
}
