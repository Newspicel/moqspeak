//! RGBA ⇄ I420 (BT.601, limited range), with box-filtered downscaling on the way in.

/// Planes of an I420 picture.
pub struct I420 {
    pub width: usize,
    pub y: Vec<u8>,
    pub u: Vec<u8>,
    pub v: Vec<u8>,
}

fn clamp(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Scales `src` (RGBA, `sw`×`sh`) to `dw`×`dh` and converts it to I420.
pub fn rgba_to_i420(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> I420 {
    let mut rgb = vec![[0f32; 3]; dw * dh];
    let sx = sw as f32 / dw as f32;
    let sy = sh as f32 / dh as f32;
    for y in 0..dh {
        let y0 = (y as f32 * sy) as usize;
        let y1 = (((y + 1) as f32 * sy) as usize).clamp(y0 + 1, sh);
        for x in 0..dw {
            let x0 = (x as f32 * sx) as usize;
            let x1 = (((x + 1) as f32 * sx) as usize).clamp(x0 + 1, sw);
            // Average up to a 3×3 neighbourhood: enough to keep text from shimmering.
            let ys = (y1 - y0).min(3);
            let xs = (x1 - x0).min(3);
            let mut acc = [0f32; 3];
            for yy in 0..ys {
                let row = (y0 + yy * (y1 - y0) / ys) * sw;
                for xx in 0..xs {
                    let i = (row + x0 + xx * (x1 - x0) / xs) * 4;
                    acc[0] += src[i] as f32;
                    acc[1] += src[i + 1] as f32;
                    acc[2] += src[i + 2] as f32;
                }
            }
            let n = (ys * xs) as f32;
            rgb[y * dw + x] = [acc[0] / n, acc[1] / n, acc[2] / n];
        }
    }

    let mut y_plane = vec![0u8; dw * dh];
    for (i, p) in rgb.iter().enumerate() {
        y_plane[i] = clamp(16.0 + 0.257 * p[0] + 0.504 * p[1] + 0.098 * p[2]);
    }
    let (cw, ch) = (dw.div_ceil(2), dh.div_ceil(2));
    let mut u = vec![0u8; cw * ch];
    let mut v = vec![0u8; cw * ch];
    for cy in 0..ch {
        for cx in 0..cw {
            let mut acc = [0f32; 3];
            let mut n = 0.0;
            for dy in 0..2 {
                for dx in 0..2 {
                    let (x, y) = (cx * 2 + dx, cy * 2 + dy);
                    if x < dw && y < dh {
                        let p = rgb[y * dw + x];
                        acc[0] += p[0];
                        acc[1] += p[1];
                        acc[2] += p[2];
                        n += 1.0;
                    }
                }
            }
            let (r, g, b) = (acc[0] / n, acc[1] / n, acc[2] / n);
            u[cy * cw + cx] = clamp(128.0 - 0.148 * r - 0.291 * g + 0.439 * b);
            v[cy * cw + cx] = clamp(128.0 + 0.439 * r - 0.368 * g - 0.071 * b);
        }
    }
    I420 {
        width: dw,
        y: y_plane,
        u,
        v,
    }
}

/// Converts I420 planes with the given strides to RGBA.
#[allow(clippy::too_many_arguments)]
pub fn i420_to_rgba(
    w: usize,
    h: usize,
    y: &[u8],
    y_stride: usize,
    u: &[u8],
    v: &[u8],
    uv_stride: usize,
    out: &mut Vec<u8>,
) {
    out.resize(w * h * 4, 255);
    for row in 0..h {
        for col in 0..w {
            let yy = y[row * y_stride + col] as f32 - 16.0;
            let uu = u[(row / 2) * uv_stride + col / 2] as f32 - 128.0;
            let vv = v[(row / 2) * uv_stride + col / 2] as f32 - 128.0;
            let i = (row * w + col) * 4;
            out[i] = clamp(1.164 * yy + 1.596 * vv);
            out[i + 1] = clamp(1.164 * yy - 0.392 * uu - 0.813 * vv);
            out[i + 2] = clamp(1.164 * yy + 2.017 * uu);
            out[i + 3] = 255;
        }
    }
}
