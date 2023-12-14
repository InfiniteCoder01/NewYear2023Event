use super::tetromino::Tetromino;
use batbox_la::*;
use bidivec::BidiVec;
use rand::Rng;
use std::time::{Duration, Instant};

pub type Color = (f64, f64, f64);

#[derive(Clone, Debug)]
pub struct Board {
    pub size: vec2<usize>,
    pub field: BidiVec<Option<(f64, f64, f64)>>,
}

impl Board {
    pub fn new(size: vec2<usize>) -> Self {
        Self {
            size,
            field: BidiVec::with_elem(None, size.x, size.y),
        }
    }

    pub fn get(&self, pos: vec2<usize>) -> Option<(f64, f64, f64)> {
        self.field.get(pos.x, pos.y).copied().flatten()
    }

    pub fn set(&mut self, pos: vec2<usize>, to: Option<(f64, f64, f64)>) {
        if let Some(cell) = self.field.get_mut(pos.x, pos.y) {
            *cell = to;
        }
    }

    pub fn shift(
        &mut self,
        origin: Option<usize>,
        offset: isize,
        filler: impl Fn() -> Option<(f64, f64, f64)>,
    ) {
        let origin = origin.unwrap_or(self.size.y - 1);
        if offset < 0 {
            for y in 0..=origin {
                let target_y = y as isize + offset;
                if target_y >= 0 {
                    for x in 0..self.size.x {
                        self.set(vec2(x, target_y as _), self.get(vec2(x, y)));
                    }
                }
                if y > (origin as isize + offset) as usize {
                    for x in 0..self.size.x {
                        self.set(vec2(x, y), filler());
                    }
                }
            }
        } else if offset > 0 {
            for y in (0..=origin).rev() {
                let target_y = y + offset as usize;
                if target_y <= origin {
                    for x in 0..self.size.x {
                        self.set(vec2(x, target_y), self.get(vec2(x, y)));
                    }
                }
                if y < offset as usize {
                    for x in 0..self.size.x {
                        self.set(vec2(x, y), filler());
                    }
                }
            }
        }
    }

    pub fn garbage(&mut self, lines: usize) {
        self.shift(None, -(lines as isize), || {
            rand::thread_rng().gen_ratio(70, 100).then(|| {
                let color = rand::thread_rng().gen_range(0.4..0.6);
                (color, color, color)
            })
        })
    }

    pub fn full_lines(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.size.1).filter(|&y| {
            for x in 0..self.size.x {
                if self.get(vec2(x, y)).is_none() {
                    return false;
                }
            }
            true
        })
    }

    pub fn draw(&mut self, context: &cairo::Context, tile: f64, offset: vec2<f64>) {
        context.set_source_rgb(0.0, 0.2, 1.0);
        context.set_line_width(4.0);
        context.rectangle(
            offset.x,
            offset.y,
            self.size.x as f64 * tile,
            self.size.y as f64 * tile,
        );
        context.stroke().unwrap();

        context.set_source_rgb(0.0, 0.1, 0.5);
        context.set_line_width(1.0);
        for y in 0..self.size.y {
            for x in 0..self.size.x {
                context.rectangle(
                    offset.x + x as f64 * tile,
                    offset.y + y as f64 * tile,
                    tile,
                    tile,
                );
                context.stroke().unwrap();
            }
        }

        for y in 0..self.size.y {
            for x in 0..self.size.x {
                if let Some(block) = self.get(vec2(x, y)) {
                    context.set_source_rgb(block.0, block.1, block.2);
                    context.rectangle(
                        offset.x + x as f64 * tile,
                        offset.y + y as f64 * tile,
                        tile,
                        tile,
                    );
                    context.fill().unwrap();
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub board: Board,
    pub tetromino: Tetromino,
    pub placed: bool,

    pub timer: Instant,
    pub move_time: Duration,
    pub general_move_time: Duration,

    pub effect: Instant,
    pub effect_time: Duration,
}

impl Game {
    pub fn new(size: vec2<usize>) -> Self {
        Self {
            board: Board::new(size),
            tetromino: Tetromino::random(size.x / 2),
            placed: false,

            timer: Instant::now(),
            move_time: Duration::from_millis(500),
            general_move_time: Duration::from_millis(500),

            effect: Instant::now() - Duration::from_secs(60 * 60),
            effect_time: Duration::from_secs(20),
        }
    }

    pub fn frame(
        &mut self,
        context: &cairo::Context,
        tile: f64,
        offset: vec2<f64>,
        opponent: Option<&mut Game>,
    ) -> bool {
        let effect = self.effect.elapsed() < self.effect_time;
        if self.timer.elapsed() >= self.move_time && !effect {
            self.timer = std::time::Instant::now();
            if !self.tetromino.try_move(&self.board, vec2(0, 1)) {
                self.tetromino.place(&mut self.board);
                self.placed = true;
            }
        }
        if self.placed {
            self.placed = false;
            self.tetromino = Tetromino::random(self.board.size.x / 2);
            if !self.tetromino.fit(&self.board) {
                return false;
            }
        }

        {
            let cleared_lines = self.board.full_lines().collect::<Vec<_>>();
            for &y in &cleared_lines {
                self.board.shift(Some(y), 1, || None);
                if effect {
                    self.board.shift(None, -1, || Some((1.0, 1.0, 1.0)));
                }
            }

            if !effect {
                if let Some(opponent) = opponent {
                    opponent.board.garbage(cleared_lines.len() / 4);
                    if cleared_lines.len() >= 8 {
                        opponent.tetromino = opponent.tetromino.clone().scale(2);
                    }
                }
            }
        }

        // self.tetromino.ai(&mut self.board);

        self.board.draw(context, tile, offset);
        self.tetromino.draw(&context, tile, offset);
        let mut shadow = self.tetromino.clone();
        shadow.drop(&self.board);
        shadow.draw_shadow(context, tile, offset);
        true
    }

    pub fn try_move(&mut self, direction: i8) -> bool {
        self.tetromino.try_move(&self.board, vec2(direction, 0))
    }

    pub fn try_turn(&mut self, ccw: bool) -> bool {
        self.tetromino.try_turn(&self.board, ccw)
    }

    pub fn speedup(&mut self, speedup: bool) {
        if speedup {
            if self.effect.elapsed() < self.effect_time {
                self.tetromino.drop(&self.board);
                self.tetromino.place(&mut self.board);
                self.placed = true;
            }
            self.move_time = Duration::from_millis(50);
        } else {
            self.move_time = self.general_move_time;
        }
    }
}
