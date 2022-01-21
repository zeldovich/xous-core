use graphics_server::api::GlyphStyle;
use graphics_server::*;
use core::fmt::Write;
use num_traits::*;

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum TestType {
    BoundingBox = 0,
    LowerRight = 1,
    LowerLeft = 2,
    TopLeft = 3,
    TopRight = 4,
    Overflow = 5,
    End = 6,
}
const TEST_STYLE: GlyphStyle = GlyphStyle::Large;
pub fn tests() {
    let _ = std::thread::spawn({
        move || {
            let xns = xous_names::XousNames::new().unwrap();
            let gfx = graphics_server::Gfx::new(&xns).unwrap();
            let ticktimer = ticktimer_server::Ticktimer::new().expect("Couldn't connect to Ticktimer");

            for index in TestType::BoundingBox.to_usize().unwrap()..TestType::End.to_usize().unwrap() {
                // show a black screen
                ticktimer.sleep_ms(1000).unwrap();
                // draw a black screen
                let screensize = gfx.screen_size().expect("Couldn't get screen size");
                let blackout = Rectangle::new_with_style(
                    Point::new(0, 0),
                    screensize,
                    DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1)
                );
                gfx.draw_rectangle(blackout).unwrap();
                gfx.flush().unwrap();

                // start the test
                ticktimer.sleep_ms(1000).unwrap();
                let clipping_area = Rectangle::new_coords(50, 50, 290, 450);

                let text_bounds = Rectangle::new_coords(10, 10, 240, 300);

                //let mut checkbound = clipping_area.clone(); // this checks against the final clipping area.
                let mut checkbound = text_bounds.clone(); // this is just around the bounds specified by the TV
                checkbound.translate(clipping_area.tl());
                checkbound.margin_out(Point::new(1, 1));
                checkbound.style = DrawStyle::new(PixelColor::Light, PixelColor::Light, 1);
                gfx.draw_rectangle(checkbound).unwrap();
                gfx.flush().unwrap();

                match FromPrimitive::from_usize(index) {
                    Some(TestType::BoundingBox) => {
                        let mut tv = TextView::new(Gid::new([0, 0, 0, 0]),
                        TextBounds::BoundingBox(
                            text_bounds
                        ));
                        tv.clip_rect = Some(clipping_area);
                        tv.style = TEST_STYLE;
                        tv.ellipsis = true;
                        tv.rounded_border = Some(4);
                        write!(tv, "This is a test of basic word wrapping inside a bounding box.\nThis should be a new line.\n\nTwo new lines.\nNew line\n with a leading space.\nDone.").unwrap();
                        write!(tv, "Let's add more text until it overflows this is just another test with https://github.com/samblenny/blitstr2/commit/bb7d4ab6a2d8913dcb520895a3c242c933413aae more words and words and words and whatever.").unwrap();
                        log::info!("rendering: {:?}", tv);
                        tv.insertion = None;
                        gfx.draw_textview(&mut tv).unwrap();
                        gfx.flush().unwrap();
                        log::info!("rendered: {:?}", tv);
                        ticktimer.sleep_ms(1000).unwrap();
                    }
                    Some(TestType::LowerRight) => {
                        let mut tv = TextView::new(Gid::new([0, 0, 0, 0]),
                        TextBounds::GrowableFromBr(
                            Point::new(text_bounds.br().x - 4, text_bounds.br().y - 4),
                            ((text_bounds.br().x - text_bounds.tl().x) / 2) as u16
                        ));
                        tv.clip_rect = Some(clipping_area);
                        tv.style = TEST_STYLE;
                        tv.ellipsis = true;
                        tv.rounded_border = Some(8);
                        write!(tv, "This is a test of basic word wrapping inside a bounding box.\nThis should be a new line.\n\nTwo new lines.\nNew line\n with a leading space.\nDone.").unwrap();
                        log::info!("rendering: {:?}", tv);
                        tv.insertion = Some(8);
                        gfx.draw_textview(&mut tv).unwrap();
                        gfx.flush().unwrap();
                        log::info!("rendered: {:?}", tv);
                        ticktimer.sleep_ms(1000).unwrap();
                    }
                    Some(TestType::LowerLeft) => {
                        let mut tv = TextView::new(Gid::new([0, 0, 0, 0]),
                        TextBounds::GrowableFromBl(
                            Point::new(text_bounds.tl().x + 4, text_bounds.br().y - 4),
                            ((text_bounds.br().x - text_bounds.tl().x) / 2) as u16
                        ));
                        tv.clip_rect = Some(clipping_area);
                        tv.style = TEST_STYLE;
                        tv.ellipsis = true;
                        tv.rounded_border = None;
                        tv.invert = true;
                        write!(tv, "This is a test of basic word wrapping inside a bounding box.\nThis should be a new line.\n\nTwo new lines.\nNew line\n with a leading space.\nDone.").unwrap();
                        log::info!("rendering: {:?}", tv);
                        tv.insertion = Some(0);
                        gfx.draw_textview(&mut tv).unwrap();
                        gfx.flush().unwrap();
                        log::info!("rendered: {:?}", tv);
                        ticktimer.sleep_ms(1000).unwrap();
                    }
                    Some(TestType::TopLeft) => {
                        let mut tv = TextView::new(Gid::new([0, 0, 0, 0]),
                        TextBounds::GrowableFromTl(
                            Point::new(text_bounds.tl().x + 4, text_bounds.tl().y + 4),
                            ((text_bounds.br().x - text_bounds.tl().x) / 2) as u16 + 20
                        ));
                        tv.clip_rect = Some(clipping_area);
                        tv.style = TEST_STYLE;
                        tv.ellipsis = false;
                        tv.rounded_border = Some(12);
                        write!(tv, "This is a test of basic word wrapping inside a bounding box.\nThis should be a new line.\n\nTwo new lines.\nNew line\n with a leading space.\nDone.").unwrap();
                        log::info!("rendering: {:?}", tv);
                        tv.insertion = Some(20);
                        gfx.draw_textview(&mut tv).unwrap();
                        gfx.flush().unwrap();
                        log::info!("rendered: {:?}", tv);
                        ticktimer.sleep_ms(1000).unwrap();
                    }
                    Some(TestType::TopRight) => {
                        let mut tv = TextView::new(Gid::new([0, 0, 0, 0]),
                        TextBounds::GrowableFromTr(
                            Point::new(text_bounds.tr().x - 4, text_bounds.tl().y + 4),
                            ((text_bounds.br().x - text_bounds.tl().x) / 2) as u16 + 30
                        ));
                        tv.clip_rect = Some(clipping_area);
                        tv.style = TEST_STYLE;
                        tv.ellipsis = false;
                        tv.rounded_border = None;
                        write!(tv, "This is a test of basic word wrapping inside a bounding box.\nThis should be a new line.\n\nTwo new lines.\nNew line\n with a leading space.\nDone.").unwrap();
                        log::info!("rendering: {:?}", tv);
                        tv.insertion = Some(21);
                        gfx.draw_textview(&mut tv).unwrap();
                        gfx.flush().unwrap();
                        log::info!("rendered: {:?}", tv);
                        ticktimer.sleep_ms(1000).unwrap();
                    }
                    Some(TestType::Overflow) => {
                        //gfx.draw_rectangle(blackout).unwrap();
                        //gfx.flush().unwrap();

                        for x_off in 0..32 {
                            let clipping_area = Rectangle::new_coords(100, 100, 200, 200);
                            let text_bounds = Rectangle::new_coords(-20 + x_off, -20 + x_off, 150, 150);

                            let mut checkbound = clipping_area.clone(); // this checks against the final clipping area.
                            checkbound.margin_out(Point::new(1, 1));
                            checkbound.style = DrawStyle::new(PixelColor::Light, PixelColor::Dark, 1);
                            gfx.draw_rectangle(checkbound).unwrap();
                            gfx.flush().unwrap();

                            let mut tv = TextView::new(Gid::new([0, 0, 0, 0]),
                            TextBounds::BoundingBox(
                                text_bounds
                            ));
                            tv.clip_rect = Some(clipping_area);
                            tv.style = TEST_STYLE;
                            tv.ellipsis = false;
                            tv.rounded_border = None;
                            write!(tv, "This is a test of basic word wrapping inside a bounding box.\nThis should be a new line.\n\nTwo new lines.\nNew line\n with a leading space.\nDone.").unwrap();
                            log::info!("rendering: {:?}", tv);
                            tv.insertion = Some(21);
                            gfx.draw_textview(&mut tv).unwrap();
                            gfx.flush().unwrap();
                            log::info!("rendered: {:?}", tv);
                            ticktimer.sleep_ms(200).unwrap();
                        }
                    }
                    _ => {}
                }
            }
        }
    });
}