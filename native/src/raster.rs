//! Small, bounded RGBA rasters for cached SDL panel textures.
//!
//! Bytes are straight-alpha RGBA, which is SDL ABGR8888 in little-endian memory.
//! Four vertical coverage samples antialias only curved boundary pixels. Interior
//! spans use packed four-byte fills; no per-pixel distance or square root runs
//! through the solid interior. Outline coverage is outer minus inner coverage,
//! never a stack of overlapping translucent draws.

const MAX_WIDTH: i32 = 1920;
const MAX_HEIGHT: i32 = 1080;
const MAX_BYTES: usize = 1920 * 1080 * 4;
const SAMPLES: usize = 4;

/// Extend image-edge RGB into transparent contain letterboxing before linear
/// texture filtering. Alpha remains zero outside the image, and every original
/// image pixel (including its rounded alpha mask) remains unchanged.
///
/// Straight-alpha filtering must interpolate the image color, rather than black,
/// at the boundary. Invalid dimensions, lengths or fit rectangles are a no-op.
pub fn bleed_contained_rgb(
    pixels: &mut [u8],
    w: usize,
    h: usize,
    ox: usize,
    oy: usize,
    iw: usize,
    ih: usize,
) {
    if w == 0 || h == 0 || w > MAX_WIDTH as usize || h > MAX_HEIGHT as usize || iw == 0 || ih == 0 {
        return;
    }
    let Some(length) = w.checked_mul(h).and_then(|n| n.checked_mul(4)) else {
        return;
    };
    let Some(right) = ox.checked_add(iw).filter(|&right| right <= w) else {
        return;
    };
    let Some(bottom) = oy.checked_add(ih).filter(|&bottom| bottom <= h) else {
        return;
    };
    if length > MAX_BYTES
        || pixels.len() != length
        || (ox == 0 && oy == 0 && right == w && bottom == h)
    {
        return;
    }
    for y in 0..h {
        let source_y = y.clamp(oy, bottom - 1);
        let spans = if y >= oy && y < bottom {
            [(0, ox), (right, w)]
        } else {
            [(0, w), (0, 0)]
        };
        for (start, end) in spans {
            for x in start..end {
                let destination = (y * w + x) * 4;
                if pixels[destination + 3] != 0 {
                    continue;
                }
                let source = (source_y * w + x.clamp(ox, right - 1)) * 4;
                let rgb = [pixels[source], pixels[source + 1], pixels[source + 2]];
                pixels[destination..destination + 3].copy_from_slice(&rgb);
            }
        }
    }
}

struct CurveRow {
    left: [f32; SAMPLES],
    width: f32,
    start: usize,
    full: usize,
}

impl CurveRow {
    fn new(y: i32, width: i32, height: i32, radius: i32, inset: i32) -> Option<Self> {
        if inset * 2 >= width || inset * 2 >= height || y < inset || y >= height - inset {
            return None;
        }
        let radius = (radius - inset).max(0) as f32;
        let inset_f = inset as f32;
        let mut left = [inset_f; SAMPLES];
        let top_center = inset_f + radius;
        let bottom_center = height as f32 - inset_f - radius;
        if radius > 0.0 && ((y as f32) < top_center || (y + 1) as f32 > bottom_center) {
            for (sample, edge) in left.iter_mut().enumerate() {
                let sy = y as f32 + (sample as f32 + 0.5) / SAMPLES as f32;
                let dy = (top_center - sy).max(sy - bottom_center).max(0.0);
                *edge = inset_f + radius - (radius * radius - dy * dy).max(0.0).sqrt();
            }
        }
        let min = left.iter().copied().fold(f32::INFINITY, f32::min);
        let max = left.iter().copied().fold(0.0, f32::max);
        let half = (width as usize + 1) / 2;
        Some(Self {
            left,
            width: width as f32,
            start: (min.floor().max(0.0) as usize).min(half),
            full: (max.ceil().max(0.0) as usize).min(half),
        })
    }

    fn coverage(&self, x: usize) -> f32 {
        let x = x as f32;
        self.left
            .iter()
            .map(|left| ((self.width - left).min(x + 1.0) - left.max(x)).clamp(0.0, 1.0))
            .sum::<f32>()
            / SAMPLES as f32
    }
}

fn fill(row: &mut [u8], start: usize, end: usize, color: [u8; 4]) {
    if end > start {
        // The pixel array fill permits packed/vectorized stores without an
        // alignment assumption about Vec<u8> or an unsafe allocation cast.
        let (pixels, tail) = row[start * 4..end * 4].as_chunks_mut::<4>();
        debug_assert!(tail.is_empty());
        pixels.fill(color);
    }
}

fn edge(row: &mut [u8], x: usize, outer: &CurveRow, inner: Option<&CurveRow>, color: [u8; 4]) {
    let coverage =
        (outer.coverage(x) - inner.map_or(0.0, |inner| inner.coverage(x))).clamp(0.0, 1.0);
    if coverage <= 0.0 {
        return;
    }
    let mut pixel = color;
    pixel[3] = (color[3] as f32 * coverage + 0.5).floor() as u8;
    row[x * 4..x * 4 + 4].copy_from_slice(&pixel);
    let other = row.len() / 4 - 1 - x;
    if other != x {
        row[other * 4..other * 4 + 4].copy_from_slice(&pixel);
    }
}

/// Rasterize a rounded rectangle or an inset outline, with a vertical gradient.
///
/// Invalid/oversized dimensions or an allocation failure return `None`. Negative
/// radius/stroke values clamp to zero, radius clamps to half the smaller side,
/// and a stroke which consumes the whole shape produces the filled outer shape.
/// `stroke == 0` fills the shape; positive strokes leave a transparent hole.
pub fn rounded(
    w: i32,
    h: i32,
    r: i32,
    stroke: i32,
    top: [u8; 4],
    bottom: [u8; 4],
) -> Option<Vec<u8>> {
    if !(1..=MAX_WIDTH).contains(&w) || !(1..=MAX_HEIGHT).contains(&h) {
        return None;
    }
    let length = (w as usize).checked_mul(h as usize)?.checked_mul(4)?;
    if length > MAX_BYTES {
        return None;
    }
    let mut result = Vec::new();
    result.try_reserve_exact(length).ok()?;
    result.resize(length, 0);
    let radius = r.clamp(0, w.min(h) / 2);
    let stroke = stroke.max(0).min(w.min(h));
    let denominator = (h - 1).max(1) as u32;
    for y in 0..h {
        let mut color = [0; 4];
        for channel in 0..4 {
            let value =
                top[channel] as u32 * (denominator - y as u32) + bottom[channel] as u32 * y as u32;
            color[channel] = ((value + denominator / 2) / denominator) as u8;
        }
        let offset = y as usize * w as usize * 4;
        let row = &mut result[offset..offset + w as usize * 4];
        let outer = CurveRow::new(y, w, h, radius, 0)?;
        let inner = if stroke > 0 {
            CurveRow::new(y, w, h, radius, stroke)
        } else {
            None
        };
        if let Some(inner) = inner.as_ref() {
            if inner.start > outer.full {
                fill(row, outer.full, inner.start, color);
                fill(
                    row,
                    w as usize - inner.start,
                    w as usize - outer.full,
                    color,
                );
            }
        } else {
            fill(row, outer.full, w as usize - outer.full, color);
        }
        for x in outer.start..outer.full {
            edge(row, x, &outer, inner.as_ref(), color);
        }
        if let Some(inner) = inner.as_ref() {
            for x in inner.start.max(outer.full)..inner.full {
                edge(row, x, &outer, Some(inner), color);
            }
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(bytes: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
        bytes[(y * w + x) * 4..(y * w + x + 1) * 4]
            .try_into()
            .unwrap()
    }

    #[test]
    fn letterbox_filtering_preserves_white_and_blue_brand_edges() {
        for brand in [[255, 255, 255], [0, 169, 225]] {
            let mut bytes = vec![0; 4 * 2 * 4];
            for y in 0..2 {
                for x in 1..3 {
                    bytes[(y * 4 + x) * 4..(y * 4 + x) * 4 + 4]
                        .copy_from_slice(&[brand[0], brand[1], brand[2], 255]);
                }
            }
            let filtered = |left: [u8; 4], right: [u8; 4]| {
                let alpha = (left[3] as f32 + right[3] as f32) / 510.0;
                std::array::from_fn::<u8, 3, _>(|c| {
                    let color = (left[c] as f32 + right[c] as f32) / 2.0;
                    (color * alpha + brand[c] as f32 * (1.0 - alpha)).round() as u8
                })
            };
            assert_ne!(
                filtered(pixel(&bytes, 4, 0, 0), pixel(&bytes, 4, 1, 0)),
                brand
            );
            bleed_contained_rgb(&mut bytes, 4, 2, 1, 0, 2, 2);
            assert_eq!(
                filtered(pixel(&bytes, 4, 0, 0), pixel(&bytes, 4, 1, 0)),
                brand
            );
            assert_eq!(pixel(&bytes, 4, 0, 0), [brand[0], brand[1], brand[2], 0]);
            assert_eq!(pixel(&bytes, 4, 3, 1), [brand[0], brand[1], brand[2], 0]);
        }
    }

    #[test]
    fn letterbox_bleed_preserves_image_rgba_and_all_outside_alpha() {
        let mut bytes = vec![0; 5 * 5 * 4];
        for y in 1..4 {
            for x in 1..4 {
                let at = (y * 5 + x) * 4;
                bytes[at..at + 4].copy_from_slice(&[
                    x as u8 * 40,
                    y as u8 * 50,
                    123,
                    if x == y { 0 } else { 96 },
                ]);
            }
        }
        bytes[(2 * 5) * 4..(2 * 5) * 4 + 4].copy_from_slice(&[77, 88, 99, 12]);
        let before = bytes.clone();
        bleed_contained_rgb(&mut bytes, 5, 5, 1, 1, 3, 3);
        for y in 0..5 {
            for x in 0..5 {
                assert_eq!(pixel(&bytes, 5, x, y)[3], pixel(&before, 5, x, y)[3]);
                if (1..4).contains(&x) && (1..4).contains(&y) {
                    assert_eq!(pixel(&bytes, 5, x, y), pixel(&before, 5, x, y));
                }
            }
        }
        assert_eq!(pixel(&bytes, 5, 0, 0), [40, 50, 123, 0]);
        assert_eq!(pixel(&bytes, 5, 4, 4), [120, 150, 123, 0]);
        assert_eq!(pixel(&bytes, 5, 0, 2), [77, 88, 99, 12]);
    }

    #[test]
    fn invalid_letterbox_bounds_are_a_noop() {
        let mut bytes = vec![7; 4 * 4 * 4];
        let before = bytes.clone();
        for (w, h, ox, oy, iw, ih) in [
            (0, 4, 0, 0, 1, 1),
            (4, 4, 0, 0, 0, 1),
            (4, 4, 0, 0, 1, 0),
            (4, 4, 3, 0, 2, 2),
            (4, 4, 0, 3, 2, 2),
            (4, 4, usize::MAX, 0, 2, 2),
            (usize::MAX, usize::MAX, 0, 0, 1, 1),
            (4, 3, 0, 0, 1, 1),
        ] {
            bleed_contained_rgb(&mut bytes, w, h, ox, oy, iw, ih);
            assert_eq!(bytes, before);
        }
    }

    #[test]
    fn circle_has_clear_corners_solid_center_and_coverage_edges() {
        let bytes = rounded(8, 8, 4, 0, [20, 40, 60, 128], [20, 40, 60, 128]).unwrap();
        assert_eq!(pixel(&bytes, 8, 0, 0), [0; 4]);
        assert_eq!(pixel(&bytes, 8, 3, 3), [20, 40, 60, 128]);
        let edge = pixel(&bytes, 8, 2, 0);
        assert_eq!(&edge[..3], &[20, 40, 60]);
        assert!(edge[3] > 0 && edge[3] < 128);
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(pixel(&bytes, 8, x, y), pixel(&bytes, 8, 7 - x, y));
                assert_eq!(pixel(&bytes, 8, x, y), pixel(&bytes, 8, x, 7 - y));
            }
        }
    }

    #[test]
    fn outline_has_a_hole_and_never_accumulates_alpha() {
        let bytes = rounded(20, 16, 6, 2, [10, 20, 30, 96], [10, 20, 30, 96]).unwrap();
        assert_eq!(pixel(&bytes, 20, 10, 8), [0; 4]);
        assert_eq!(pixel(&bytes, 20, 0, 8), [10, 20, 30, 96]);
        assert!(bytes.chunks_exact(4).all(|p| p[3] <= 96));
        // Where both curved boundaries antialias in the same pixel, its alpha
        // is the difference in coverage, rather than two blended shapes.
        let outer = rounded(20, 16, 6, 0, [10, 20, 30, 96], [10, 20, 30, 96]).unwrap();
        assert!(bytes
            .chunks_exact(4)
            .zip(outer.chunks_exact(4))
            .all(|(a, b)| a[3] <= b[3]));
        let square = rounded(5, 5, 0, 1, [1, 2, 3, 255], [1, 2, 3, 255]).unwrap();
        assert_eq!(pixel(&square, 5, 0, 0), [1, 2, 3, 255]);
        assert_eq!(pixel(&square, 5, 1, 1), [0; 4]);
    }

    #[test]
    fn gradient_preserves_endpoints_and_straight_rgba() {
        let top = [10, 20, 30, 40];
        let bottom = [90, 80, 70, 60];
        let bytes = rounded(5, 4, 0, 0, top, bottom).unwrap();
        assert_eq!(pixel(&bytes, 5, 2, 0), top);
        assert_eq!(pixel(&bytes, 5, 2, 1), [37, 40, 43, 47]);
        assert_eq!(pixel(&bytes, 5, 2, 3), bottom);
        assert_eq!(rounded(1, 1, 0, 0, top, bottom).unwrap(), top);
    }

    #[test]
    fn geometry_clamps_and_dimensions_are_bounded() {
        let c = [1, 2, 3, 255];
        assert_eq!(rounded(7, 5, i32::MAX, 0, c, c), rounded(7, 5, 2, 0, c, c));
        assert_eq!(rounded(7, 5, -8, -2, c, c), rounded(7, 5, 0, 0, c, c));
        assert_eq!(rounded(7, 5, 2, i32::MAX, c, c), rounded(7, 5, 2, 0, c, c));
        for (w, h) in [
            (0, 1),
            (1, 0),
            (-1, 10),
            (1921, 1),
            (1, 1081),
            (i32::MAX, i32::MAX),
        ] {
            assert!(rounded(w, h, 0, 0, c, c).is_none());
        }
        assert_eq!(rounded(1920, 1080, 0, 0, c, c).unwrap().len(), MAX_BYTES);
    }
}
