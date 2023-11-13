use fontdue::layout::Layout;
use rayon::{prelude::*, slice::ParallelSliceMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const WHITE: Color = Color::from_rgb(0xffffff);
    pub const BLACK: Color = Color::from_rgb(0x000000);
    pub const RED: Color = Color::from_rgb(0xff0000);
    pub const GREEN: Color = Color::from_rgb(0x00ff00);
    pub const BLUE: Color = Color::from_rgb(0x0000ff);
    pub const YELLOW: Color = Color::from_rgb(0xffff00);
    pub const CYAN: Color = Color::from_rgb(0x00ffff);
    pub const MAGENTA: Color = Color::from_rgb(0xff00ff);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn from_rgb(rgb: u32) -> Self {
        let r = (rgb >> 16) & 0xff;
        let g = (rgb >> 8) & 0xff;
        let b = rgb & 0xff;
        Self::new(r as _, g as _, b as _)
    }

    pub const fn grayscale(color: u8) -> Self {
        Self::new(color, color, color)
    }

    /// Sets the pixel of this [`Color`].
    ///
    /// # Safety
    /// Use this only in [`Frame::parallel_region`]
    /// .
    #[inline(always)]
    pub unsafe fn set_pixel(self, pixel: &mut [u8]) {
        *pixel.get_unchecked_mut(0) = self.r;
        *pixel.get_unchecked_mut(1) = self.g;
        *pixel.get_unchecked_mut(2) = self.b;
    }

    /// Blends the pixel of this [`Color`] with the pixel of the given [`Color`] by the given alpha value.
    ///
    /// # Safety
    /// Use this only in [`Frame::parallel_region`]
    #[inline(always)]
    #[rustfmt::skip]
    pub unsafe fn blend_pixel(&self, pixel: &mut [u8], alpha: u8) {
        *pixel.get_unchecked_mut(0) = (*pixel.get_unchecked(0) as u32 * (255 - alpha as u32) / 255 + self.r as u32 * alpha as u32) as _;
        *pixel.get_unchecked_mut(1) = (*pixel.get_unchecked(1) as u32 * (255 - alpha as u32) / 255 + self.g as u32 * alpha as u32) as _;
        *pixel.get_unchecked_mut(2) = (*pixel.get_unchecked(2) as u32 * (255 - alpha as u32) / 255 + self.b as u32 * alpha as u32) as _;
    }
}

struct ThreadPtr<T>(*mut T);
unsafe impl<T> Send for ThreadPtr<T> {}
unsafe impl<T> Sync for ThreadPtr<T> {}

impl<T> Clone for ThreadPtr<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

// * ------------------------------------- Frame ------------------------------------ * //
pub struct Frame<'a> {
    buffer: &'a mut [u8],
    pub width: usize,
    pub height: usize,
}

impl<'a> Frame<'a> {
    pub fn new(buffer: &'a mut [u8], width: usize, height: usize) -> Self {
        Self {
            buffer,
            width,
            height,
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.buffer.par_chunks_exact_mut(3).for_each(|pixel| {
            pixel[0] = color.r;
            pixel[1] = color.g;
            pixel[2] = color.b;
        });
    }

    /// This scary function just calls shader on each pixel of the region in parallel
    #[inline(always)]
    pub fn parallel_region(
        &mut self,
        x0: i32,
        y0: i32,
        width: usize,
        height: usize,
        shader: impl Fn(usize, usize, &mut [u8]) + Sync + Send,
    ) {
        let (x, y) = (x0.max(0) as usize, y0.max(0) as usize);
        let x = x.min(self.width - 1);

        let buffer = ThreadPtr(self.buffer.as_mut_ptr());

        (y.min(self.height - 1)..(y + height).min(self.height))
            .into_par_iter()
            .for_each(|y| {
                let index = y * self.width + x;
                let row = unsafe {
                    std::slice::from_raw_parts_mut(
                        buffer.clone().0.add(index * 3),
                        width.min(self.width - x) * 3,
                    )
                };
                let y = (y as i32 - y0) as _;
                row.par_chunks_exact_mut(3)
                    .enumerate()
                    .for_each(|(x, pixel)| shader(x, y, pixel));
            });
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: usize, height: usize, color: Color) {
        self.parallel_region(x, y, width, height, |_, _, pixel| unsafe {
            color.set_pixel(pixel);
        });
    }

    pub fn draw_layout_text(
        &mut self,
        x: i32,
        y: i32,
        layout: &Layout,
        color: Color,
        fonts: &[fontdue::Font],
    ) {
        let frame = ThreadPtr(self as *mut Self);

        layout.glyphs().par_iter().for_each(|glyph| {
            let (metrics, bitmap) =
                fonts[glyph.font_index].rasterize_indexed(glyph.key.glyph_index, glyph.key.px);
            unsafe { &mut (*frame.clone().0) }.parallel_region(
                x + glyph.x as i32,
                y + glyph.y as i32,
                metrics.width,
                metrics.height,
                |u, v, pixel| unsafe {
                    color.blend_pixel(pixel, bitmap[u + v * metrics.width]);
                },
            );
        });
    }

    pub fn draw_text(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        color: Color,
        size: f32,
        fonts: &[fontdue::Font],
    ) {
        let mut layout = Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown);
        layout.append(fonts, &fontdue::layout::TextStyle::new(text, size, 0));
        self.draw_layout_text(x, y, &layout, color, fonts);
    }
}
