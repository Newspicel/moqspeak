//! A small safe wrapper over rav1d's dav1d-compatible API.

use std::mem::MaybeUninit;
use std::ptr::NonNull;

use anyhow::{Result, anyhow};
use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
use rav1d::include::dav1d::headers::DAV1D_PIXEL_LAYOUT_I420;
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{
    dav1d_close, dav1d_data_create, dav1d_default_settings, dav1d_get_picture, dav1d_open,
    dav1d_picture_unref, dav1d_send_data,
};

use super::color::i420_to_rgba;

/// One AV1 decoder. Feed it a frame's OBUs, get an RGBA picture back.
pub struct Decoder {
    ctx: Option<Dav1dContext>,
    rgba: Vec<u8>,
}

// SAFETY: the context is only ever used through `&mut self`.
unsafe impl Send for Decoder {}

impl Decoder {
    pub fn new() -> Result<Self> {
        let mut settings = MaybeUninit::<Dav1dSettings>::uninit();
        let mut ctx: Option<Dav1dContext> = None;
        // SAFETY: dav1d_default_settings fills every field; dav1d_open writes the context.
        unsafe {
            dav1d_default_settings(NonNull::new(settings.as_mut_ptr()).unwrap());
            let mut settings = settings.assume_init();
            settings.n_threads = 2;
            settings.max_frame_delay = 1;
            let r = dav1d_open(NonNull::new(&mut ctx), NonNull::new(&mut settings));
            if r.0 < 0 {
                return Err(anyhow!("dav1d_open failed ({})", r.0));
            }
        }
        Ok(Self {
            ctx,
            rgba: Vec::new(),
        })
    }

    /// Decodes one frame. Returns the picture as RGBA when one is ready.
    pub fn decode(&mut self, obus: &[u8]) -> Result<Option<(u32, u32, Vec<u8>)>> {
        // SAFETY: the data buffer is created by dav1d at the right size and filled before use;
        // dav1d takes ownership of it in send_data. Pictures are unref'd after conversion.
        unsafe {
            let mut data = Dav1dData::default();
            let buf = dav1d_data_create(NonNull::new(&mut data), obus.len());
            if buf.is_null() {
                return Err(anyhow!("dav1d_data_create failed"));
            }
            std::ptr::copy_nonoverlapping(obus.as_ptr(), buf, obus.len());
            let r = dav1d_send_data(self.ctx.clone(), NonNull::new(&mut data));
            if r.0 < 0 && data.sz > 0 {
                // The decoder is full: drain a picture, then retry once.
                let picture = self.take_picture();
                let _ = dav1d_send_data(self.ctx.clone(), NonNull::new(&mut data));
                if picture.is_some() {
                    return Ok(picture);
                }
            }
            Ok(self.take_picture())
        }
    }

    unsafe fn take_picture(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        let mut pic = Dav1dPicture::default();
        // SAFETY: the caller holds a live context; the picture is released below.
        let r = unsafe { dav1d_get_picture(self.ctx.clone(), NonNull::new(&mut pic)) };
        if r.0 < 0 {
            return None;
        }
        let out = if pic.p.layout == DAV1D_PIXEL_LAYOUT_I420 && pic.p.bpc == 8 {
            let (w, h) = (pic.p.w as usize, pic.p.h as usize);
            let (ys, uvs) = (pic.stride[0] as usize, pic.stride[1] as usize);
            let (cw_rows, h_rows) = (h.div_ceil(2), h);
            // SAFETY: the planes hold `stride × rows` bytes for an 8-bit I420 picture.
            let (y, u, v) = unsafe {
                (
                    std::slice::from_raw_parts(pic.data[0]?.as_ptr() as *const u8, ys * h_rows),
                    std::slice::from_raw_parts(pic.data[1]?.as_ptr() as *const u8, uvs * cw_rows),
                    std::slice::from_raw_parts(pic.data[2]?.as_ptr() as *const u8, uvs * cw_rows),
                )
            };
            i420_to_rgba(w, h, y, ys, u, v, uvs, &mut self.rgba);
            Some((w as u32, h as u32, self.rgba.clone()))
        } else {
            None
        };
        // SAFETY: releases the reference dav1d_get_picture handed out.
        unsafe { dav1d_picture_unref(NonNull::new(&mut pic)) };
        out
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        // SAFETY: closes the context opened in `new`.
        unsafe { dav1d_close(NonNull::new(&mut self.ctx)) };
    }
}
