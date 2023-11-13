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
                    std::slice::from_raw_parts_mut(buffer.clone().0, self.width * self.height * 3)
                };
                row[index * 3..(index + width.min(self.width - x)) * 3]
                    .par_chunks_exact_mut(3)
                    .enumerate()
                    .for_each(|(x, pixel)| {
                        shader((x as i32 - x0) as _, (y as i32 - y0) as _, pixel)
                    });
            });
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: usize, height: usize, color: Color) {
        self.parallel_region(x, y, width, height, |_, _, pixel| unsafe {
            *pixel.get_unchecked_mut(0) = color.r;
            *pixel.get_unchecked_mut(1) = color.g;
            *pixel.get_unchecked_mut(2) = color.b;
        });
    }

    pub fn draw_layout_text(
        &mut self,
        x: i32,
        y: i32,
        layout: &Layout,
        // color: Color,
        fonts: &[fontdue::Font],
    ) {
        let frame = ThreadPtr(self as *mut Self);

        layout.glyphs().par_iter().for_each(|glyph| {
            let (metrics, bitmap) = fonts[glyph.font_index]
                .rasterize_indexed_subpixel(glyph.key.glyph_index, glyph.key.px);
            unsafe { &mut (*frame.clone().0) }.parallel_region(
                x,
                y,
                metrics.width,
                metrics.height,
                |u, v, pixel| {
                    let index = (u + v * metrics.width) * 3;
                    // get_unchecked_mut - 5000us
                    // *pixel.get_unchecked_mut(0) = bitmap[index];
                    // *pixel.get_unchecked_mut(1) = bitmap[index + 1];
                    // *pixel.get_unchecked_mut(2) = bitmap[index + 2];
                    pixel.copy_from_slice(&bitmap[index..index + 3]);
                },
            );
        });
    }

    // pub fn draw_text(&mut self, x: i32, y: i32, text: &str, color: Color, size: f32, font: &fontdue::Font) {
    //     let characters = text.chars().collect()
    //     let (metrics, bitmap) = font.rasterize_subpixel(, size);
    //     for y in 0..metrics.height {
    //         for x in (0..metrics.width * 3).step_by(3) {
    //             let char_r = bitmap[x + y * metrics.width * 3];
    //             let char_g = bitmap[x + 1 + y * metrics.width * 3];
    //             let char_b = bitmap[x + 2 + y * metrics.width * 3];
    //             print!("\x1B[48;2;{};{};{}m   ", char_r, char_g, char_b);
    //         }
    //         println!("\x1B[0m");
    //     }
    // }
}
