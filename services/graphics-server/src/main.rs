#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

use log::{error, info};

mod api;
use api::{Opcode, ClipObject, ClipObjectType};

use core::pin::Pin;
use rkyv::{archived_value, Unarchive, archived_value_mut};

mod backend;
use backend::XousDisplay;

mod op;

use core::convert::TryFrom;

mod logo;

use api::{DrawStyle, PixelColor, Rectangle, TextBounds, RoundedRectangle, Point};
use blitstr_ref as blitstr;

mod fontmap;

fn draw_boot_logo(display: &mut XousDisplay) {
    display.blit_screen(logo::LOGO_MAP);
}

#[cfg(target_os = "none")]
fn map_fonts() {
    //info!("GFX: mapping fonts");
    // this maps an extra page if the total length happens to fall on a 4096-byte boundary, but this is ok
    // because the reserved area is much larger
    let fontlen: u32 = ((fontmap::FONT_TOTAL_LEN as u32) & 0xFFFF_F000) + 0x1000;
    //info!("GFX: requesting map of length 0x{:08x} at 0x{:08x}", fontlen, fontmap::FONT_BASE);
    let fontregion = xous::syscall::map_memory(
        xous::MemoryAddress::new(fontmap::FONT_BASE),
        None,
        fontlen as usize,
        xous::MemoryFlags::R,
    ).expect("GFX: couldn't map fonts");
    info!("GFX: font base at virtual 0x{:08x}, len of 0x{:08x}", usize::from(fontregion.addr), usize::from(fontregion.size));

    //info!("GFX: mapping regular font to 0x{:08x}", usize::from(fontregion.addr) + fontmap::REGULAR_OFFSET as usize);
    blitstr::map_font(blitstr::GlyphData::Emoji((usize::from(fontregion.addr) + fontmap::EMOJI_OFFSET) as usize));
    blitstr::map_font(blitstr::GlyphData::Hanzi((usize::from(fontregion.addr) + fontmap::HANZI_OFFSET) as usize));
    blitstr::map_font(blitstr::GlyphData::Regular((usize::from(fontregion.addr) + fontmap::REGULAR_OFFSET) as usize));
    blitstr::map_font(blitstr::GlyphData::Small((usize::from(fontregion.addr) + fontmap::SMALL_OFFSET) as usize));
    blitstr::map_font(blitstr::GlyphData::Bold((usize::from(fontregion.addr) + fontmap::BOLD_OFFSET) as usize));
}

#[cfg(not(target_os = "none"))]
fn map_fonts() {
    // does nothing
}

#[xous::xous_main]
fn xmain() -> ! {
    let debug1 = false;
    log_server::init_wait().unwrap();
    info!("GFX: my PID is {}", xous::process::id());

    let sid = xous_names::register_name(xous::names::SERVER_NAME_GFX).expect("GFX: can't register server");
    info!("GFX: Server listening on address {:?}", sid);

    // Create a new monochrome simulator display.
    let mut display = XousDisplay::new();

    draw_boot_logo(&mut display);

    map_fonts();

    let mut current_glyph = blitstr::GlyphStyle::Regular;
    let mut current_string_clip = blitstr::ClipRect::full_screen();
    let mut current_cursor = blitstr::Cursor::from_top_left_of(current_string_clip);

    if false {
        // leave this test case around
        // for some reason, the top right quadrant draws an extra pixel inside the fill area
        // when a fill color of "Light" is specified. However, if `None` fill is specified, it
        // works correctly. This is really puzzling, because the test for filled drawing happens
        // after the test for border drawing.
        use api::Point;
        let mut r = Rectangle::new(Point::new(20, 200), Point::new(151, 301));
        r.style = DrawStyle {
            fill_color: Some(PixelColor::Light),
            stroke_color: Some(PixelColor::Dark),
            stroke_width: 1,
        };
        let rr = RoundedRectangle::new(r, 16);
        op::rounded_rectangle(display.native_buffer(), rr, None);
    }

    let screen_clip = Rectangle::new(Point::new(0,0), display.screen_size());

    display.redraw();
    loop {
        let msg = xous::receive_message(sid).unwrap();
        //info!("GFX: Message: {:?}", msg);
        if let xous::Message::Borrow(m) = &msg.body {
            let buf = unsafe { xous::XousBuffer::from_memory_message(m) };
            let bytes = Pin::new(buf.as_ref());
            let value = unsafe {
                archived_value::<api::Opcode>(&bytes, m.id as usize)
            };
            match &*value {
                rkyv::Archived::<api::Opcode>::String(rkyv_s) => {
                    let s: xous::String<4096> = rkyv_s.unarchive();
                    //info!("GFX: unarchived string: {:?}", s);
                    blitstr::paint_str(
                        display.native_buffer(),
                        current_string_clip.into(),
                        &mut current_cursor,
                        current_glyph.into(),
                        s.as_str().unwrap(),
                        false,
                        None,
                        false,
                        blitstr::xor_char
                    );
                    //info!("GFX: string painted");
                },
                rkyv::Archived::<api::Opcode>::StringXor(rkyv_s) => {
                    let s: xous::String<4096> = rkyv_s.unarchive();
                    blitstr::paint_str(
                        display.native_buffer(),
                        current_string_clip.into(),
                        &mut current_cursor,
                        current_glyph.into(),
                        s.as_str().unwrap(),
                        true,
                        None,
                        false,
                        blitstr::xor_char
                    );
                },
                rkyv::Archived::<api::Opcode>::DrawClipObject(rco) => {
                    let obj: ClipObject = rco.unarchive();
                    if debug1{info!("GFX: DrawClipObject {:?}", obj);}
                    match obj.obj {
                        ClipObjectType::Line(line) => {
                            op::line(display.native_buffer(), line, Some(obj.clip));
                        },
                        ClipObjectType::Circ(circ) => {
                            op::circle(display.native_buffer(), circ, Some(obj.clip));
                        },
                        ClipObjectType::Rect(rect) => {
                            op::rectangle(display.native_buffer(), rect, Some(obj.clip));
                        },
                        ClipObjectType::RoundRect(rr) => {
                            op::rounded_rectangle(display.native_buffer(), rr, Some(obj.clip));
                        }
                    }
                },
                _ => panic!("GFX: invalid response from server -- corruption occurred in MemoryMessage")
            };
        } else if let xous::Message::MutableBorrow(m) = &msg.body {
            let mut buf = unsafe { xous::XousBuffer::from_memory_message(m) };
            let value = unsafe {
                archived_value_mut::<api::Opcode>(Pin::new(buf.as_mut()), m.id as usize)
            };
            let debugtv: bool = false;
            match &*value {
                rkyv::Archived::<api::Opcode>::DrawTextView(rtv) => {
                    let mut tv = rtv.unarchive();

                    if tv.clip_rect.is_none() { continue } // if no clipping rectangle is specified, nothing to draw
                    let screen_offset: Point = tv.clip_rect.unwrap().tl; // this is the translation vector to and from screen space

                    let paintfn = if tv.dry_run {
                        if debugtv { info!("GFX(TV): doing dry run"); }
                        blitstr::simulate_char
                    } else {
                        if debugtv { info!("GFX(TV): doing live run"); }
                        blitstr::xor_char
                    };

                    // first compute the bounding box, if it isn't computed
                    if tv.bounds_computed.is_none() {
                        match tv.bounds_hint {
                            TextBounds::BoundingBox(r) => {
                                tv.bounds_computed = Some(r);
                            },
                            TextBounds::GrowableFromBr(_br, _width) => {
                                unimplemented!()
                            },
                            TextBounds::GrowableFromTl(_tl, _width) => {
                                unimplemented!()
                            },
                            TextBounds::GrowableFromBl(_bl, _width) => {
                                unimplemented!()
                            }
                        }
                    }
                    if debugtv { info!("GFX(TV): computed bounds {:?}", tv.bounds_computed); }

                    // clear the bounding box if requested
                    let mut clear_rect = tv.bounds_computed.unwrap();

                    // move things into screen coordinates
                    clear_rect.translate(screen_offset);

                    let bordercolor = if tv.draw_border {
                        Some(PixelColor::Dark)
                    } else {
                        None
                    };
                    let borderwidth: i16 = if tv.draw_border {
                        1
                    } else {
                        0
                    };
                    let fillcolor = if tv.clear_area {
                        Some(PixelColor::Light)
                    } else {
                        None
                    };

                    clear_rect.style = DrawStyle {
                        fill_color: fillcolor,
                        stroke_color: bordercolor,
                        stroke_width: borderwidth,
                    };
                    if tv.rounded_border.is_some() {
                        op::rounded_rectangle(display.native_buffer(),
                           RoundedRectangle::new(clear_rect, tv.rounded_border.unwrap() as _), tv.clip_rect);
                    } else {
                        if debugtv { info!("GFX(TV): clearing rectangle {:?}", clear_rect); }
                        op::rectangle(display.native_buffer(), clear_rect, tv.clip_rect);
                    }


                    // compute the final clipping region for the string
                    clear_rect.margin(tv.margin);
                    let cr = match clear_rect.clip_with(screen_clip) {
                        Some(r) => r,
                        _ => continue, // don't draw anything if somehow this doesn't fit in the creen.
                    };

                    let mut ref_cursor = blitstr::Cursor::from_top_left_of(cr.into());
                    if debugtv { info!("GFX(TV): paint_str with {:?} | {:?} | {:?} | {:?}", cr, ref_cursor, tv.style, tv.text); }
                    blitstr::paint_str(
                        display.native_buffer(),
                        cr.into(),
                        &mut ref_cursor,
                        tv.style.into(),
                        tv.text.as_str().unwrap(),
                        false,
                        tv.insertion,
                        tv.ellipsis,
                        paintfn
                    );
                    // translate the cursor return value back to canvas coordinates
                    tv.cursor = blitstr::Cursor {
                        pt: blitstr::Pt::new(
                            ref_cursor.pt.x - screen_offset.x as u32,
                            ref_cursor.pt.y - screen_offset.y as u32,
                        ),
                        line_height: ref_cursor.line_height,
                    };
                    if debugtv{info!("GFX(TV): returning cursor of {:?}", tv.cursor);}

                    // pack our data back into the buffer to return
                    use rkyv::Write;
                    let mut writer = rkyv::ArchiveBuffer::new(buf);
                    writer.archive(&api::Opcode::DrawTextView(tv)).expect("GFX: couldn't re-archive return value");
                },
                _ => panic!("GFX: invalid mutable borrow message"),
            };
        } else if let Ok(opcode) = Opcode::try_from(&msg.body) {
            let debugop = false;
            if debugop {info!("GFX: Opcode: {:?}", opcode);}
            match opcode {
                Opcode::Flush => {
                    display.update();
                    display.redraw();
                }
                Opcode::Clear => {
                    let mut r = Rectangle::full_screen();
                    r.style = DrawStyle::new(PixelColor::Light, PixelColor::Light, 0);
                    op::rectangle(display.native_buffer(), r, None)
                }
                Opcode::Line(l) => {
                    op::line(display.native_buffer(), l, None);
                }
                Opcode::Rectangle(r) => {
                    op::rectangle(display.native_buffer(), r, None);
                }
                Opcode::RoundedRectangle(rr) => {
                    op::rounded_rectangle(display.native_buffer(), rr, None);
                }
                Opcode::Circle(c) => {
                    op::circle(display.native_buffer(), c, None);
                }
                Opcode::SetGlyphStyle(glyph) => {
                    current_glyph = glyph;
                }
                Opcode::SetCursor(c) => {
                    current_cursor = c;
                }
                Opcode::GetCursor => {
                    let pt: api::Point =
                        api::Point::new(current_cursor.pt.x as i16, current_cursor.pt.y as i16);
                    xous::return_scalar2(msg.sender, pt.into(), current_cursor.line_height as usize)
                        .expect("GFX: could not return GetCursor request");
                }
                Opcode::SetStringClipping(r) => {
                    current_string_clip = r;
                }
                Opcode::ScreenSize => {
                    let pt = display.screen_size();
                    xous::return_scalar2(msg.sender, pt.x as usize, pt.y as usize)
                        .expect("GFX: couldn't return ScreenSize request");
                }
                Opcode::QueryGlyphStyle => {
                    xous::return_scalar2(
                        msg.sender,
                        current_glyph.into(),
                        blitstr::glyph_to_height_hint(current_glyph),
                    )
                    .expect("GFX: could not return QueryGlyph request");
                }
                Opcode::QueryGlyphProps(glyph) => {
                    xous::return_scalar2(
                        msg.sender,
                        glyph.into(),
                        blitstr::glyph_to_height_hint(glyph),
                    )
                    .expect("GFX: could not return QueryGlyphProps request");
                }
                /*
                Opcode::TextView(tv) => {
                    info!("GFX: got draw of '{:?}'", tv);
                    op::textview(display.native_buffer(), tv);
                }*/
                _ => panic!("GFX: received opcode scalar that is not handled")
            }
        } else {
            error!("GFX: Couldn't convert opcode");
        }
        display.update();
    }
}
