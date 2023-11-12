use rayon::{prelude::*, slice::ParallelSliceMut};

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

    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        let index = y * self.width + x;
        Color::new(
            self.buffer[index],
            self.buffer[index + 1],
            self.buffer[index + 2],
        )
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let index = y * self.width + x;
        self.buffer[index] = color.r;
        self.buffer[index + 1] = color.g;
        self.buffer[index + 2] = color.b;
    }

    pub fn clear(&mut self, color: Color) {
        self.buffer.par_chunks_exact_mut(3).for_each(|pixel| {
            pixel[0] = color.r;
            pixel[1] = color.g;
            pixel[2] = color.b;
        });
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: usize, height: usize, color: Color) {
        let (x, y) = (x.max(0) as usize, y.max(0) as usize);
        let x = x.min(self.width - 1);

        #[derive(Clone)]
        struct ThreadPtr<T>(*mut T);
        unsafe impl<T> Send for ThreadPtr<T> {}
        unsafe impl<T> Sync for ThreadPtr<T> {}

        let buffer = ThreadPtr(self.buffer.as_mut_ptr());

        (y.min(self.height - 1)..(y + height).min(self.height))
            .into_par_iter()
            .for_each(|y| {
                let index = y * self.width + x;
                let row = unsafe {
                    std::slice::from_raw_parts_mut(buffer.clone().0, width.min(self.width - x) * 3)
                };
                row[index * 3..(index + width) * 3]
                    .par_chunks_exact_mut(3)
                    .for_each(|pixel| {
                        pixel[0] = color.r;
                        pixel[1] = color.g;
                        pixel[2] = color.b;
                    });
            });
    }
}
