use super::game::*;
use batbox_la::*;
use rand::Rng;
use scheduler::*;

#[derive(Clone, Debug)]
pub struct Tetromino {
    pub pos: vec2<i32>,
    pub blocks: Vec<vec2<u8>>,
    pub size: u8,
    pub color: Color,
}

impl Tetromino {
    pub fn new(blocks: Vec<vec2<u8>>, size: u8, color: Color, pos: usize) -> Self {
        Self {
            pos: vec2(pos as _, 0),
            blocks,
            size,
            color,
        }
    }

    pub fn random(pos: usize) -> Self {
        #[rustfmt::skip]
        let shapes = [
            ([vec2(1, 0), vec2(1, 1), vec2(1, 2), vec2(2, 2)], 3), // L
            ([vec2(0, 2), vec2(1, 0), vec2(1, 1), vec2(1, 2)], 3), // J
            ([vec2(0, 0), vec2(0, 1), vec2(1, 0), vec2(1, 1)], 2), // O
            ([vec2(1, 0), vec2(0, 1), vec2(1, 1), vec2(2, 1)], 3), // T
            ([vec2(2, 0), vec2(2, 1), vec2(2, 2), vec2(2, 3)], 4), // I
            ([vec2(1, 0), vec2(2, 0), vec2(0, 1), vec2(1, 1)], 3), // S
            ([vec2(0, 0), vec2(1, 0), vec2(1, 1), vec2(2, 1)], 3), // Z
        ];

        let colors = [
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, 0.0, 1.0),
            (1.0, 1.0, 0.0),
            (0.0, 1.0, 1.0),
        ];

        let (blocks, size) = shapes[rand::thread_rng().gen_range(0..shapes.len())];
        let color = colors[rand::thread_rng().gen_range(0..colors.len())];
        Self::new(Vec::from(blocks), size, color, pos - size as usize / 2)
    }

    pub fn scale(self, scale: u8) -> Self {
        Self {
            pos: self.pos - vec2(self.pos.x - (self.size * scale / 2) as i32, 0),
            blocks: self
                .blocks
                .into_iter()
                .flat_map(|block| {
                    let mut blocks = Vec::new();
                    for y in 0..scale {
                        for x in 0..scale {
                            blocks.push(block * scale + vec2(x, y));
                        }
                    }
                    blocks
                })
                .collect(),
            size: self.size * scale,
            color: self.color,
        }
    }

    pub fn draw(&self, context: &cairo::Context, tile: f64, offset: vec2<f64>) {
        let padding = 3.0;
        context.set_source_rgb(self.color.0, self.color.1, self.color.2);
        for block in self.blocks() {
            let pos = block.map(f64::from) * tile + offset + vec2::splat(padding);
            context.rectangle(pos.x, pos.y, tile - padding * 2.0, tile - padding * 2.0);
        }
        context.fill().unwrap();
    }

    pub fn draw_shadow(&self, context: &cairo::Context, tile: f64, offset: vec2<f64>) {
        let padding = 3.0;
        context.set_source_rgb(self.color.0, self.color.1, self.color.2);
        for block in self.blocks() {
            let pos = block.map(f64::from) * tile + offset + vec2::splat(padding);
            context.rectangle(pos.x, pos.y, tile - padding * 2.0, tile - padding * 2.0);
        }
        context.stroke().unwrap();
    }

    pub fn turn(&mut self, ccw: bool) {
        for block in &mut self.blocks {
            let last = *block;
            if ccw {
                block.x = last.y;
                block.y = self.size - last.x - 1;
            } else {
                block.x = self.size - last.y - 1;
                block.y = last.x;
            }
        }
    }

    pub fn try_turn(&mut self, board: &Board, ccw: bool) -> bool {
        let mut attempt = self.clone();
        attempt.turn(ccw);
        if attempt.fits(board) {
            *self = attempt;
            true
        } else {
            false
        }
    }

    pub fn try_move(&mut self, board: &Board, direction: vec2<i8>) -> bool {
        self.pos += direction.map(i32::from);
        if !self.fits(board) {
            self.pos -= direction.map(i32::from);
            false
        } else {
            true
        }
    }

    pub fn drop(&mut self, board: &Board) {
        while self.try_move(board, vec2(0, 1)) {}
    }

    pub fn place(&self, board: &mut Board) {
        for block in self.blocks() {
            if Aabb2::from_corners(vec2::ZERO, board.size.map(|x| x as _)).contains(block) {
                board.set(block.map(|x| x as _), Some(self.color));
            }
        }
    }

    pub fn fits(&self, board: &Board) -> bool {
        for block in self.blocks() {
            if !Aabb2::from_corners(
                vec2::ZERO,
                board.size.map(|x| x as _) - vec2(0, board.zone_lines.len() as _),
            )
            .contains(block)
                || board.get(block.map(|x| x as _)).is_some()
            {
                return false;
            }
        }
        true
    }

    pub fn blocks(&self) -> impl Iterator<Item = vec2<i32>> + '_ {
        self.blocks
            .iter()
            .map(|block| self.pos + block.map(i32::from))
    }
}

impl Tetromino {
    /// I did a lot of tetris AIs. But this one I did by following this article: https://codemyroad.wordpress.com/2013/04/14/tetris-ai-the-near-perfect-player/
    pub fn ai(&mut self, board: &mut Board) -> bool {
        let pos = self.pos;
        let mut best = (self.pos.x, 0, usize::MAX, f64::MIN);
        let mut test = |tetromino: &mut Self, board: &mut Board, rotation| {
            if !tetromino.fits(board) {
                return;
            }
            tetromino.drop(board);
            tetromino.place(board);

            let complete_lines = board.full_lines().count();
            let (aggregate_height, max_height, bumpines) = {
                let mut aggregate_height = 0;
                let mut max_height = 0;
                let mut bumpines = 0;
                let mut last_height = None;
                for x in 0..board.size.x {
                    let mut height = 0;
                    for y in 0..board.size.y {
                        if board.get(vec2(x, y)).is_some()
                            || y >= board.size.y - board.zone_lines.len()
                        {
                            height = board.size.y - y;
                            break;
                        }
                    }
                    aggregate_height += height - complete_lines;
                    max_height = max_height.max(height);
                    if let Some(last_height) = last_height {
                        bumpines += height.abs_diff(last_height);
                    }
                    last_height = Some(height);
                }
                (aggregate_height, max_height, bumpines)
            };

            let holes = {
                let mut holes = 0;
                for y in 1..board.size.y - board.zone_lines.len() {
                    for x in 0..board.size.x {
                        if board.get(vec2(x, y)).is_none() && board.get(vec2(x, y - 1)).is_some() {
                            holes += 1;
                        }
                    }
                }
                holes
            };

            let score = aggregate_height as f64 * -0.510066
                + complete_lines as f64 * 0.760666
                + holes as f64 * -0.35663
                + bumpines as f64 * -0.184483;

            if score > best.3 {
                best = (tetromino.pos.x, rotation, max_height, score);
            }

            for block in tetromino.blocks() {
                board.set(block.map(|x| x as _), None);
            }
            tetromino.pos.y = pos.y;
        };

        for rotation in 0..4 {
            test(self, board, rotation);
            while self.try_move(board, vec2(-1, 0)) {
                test(self, board, rotation);
            }
            self.pos.x = pos.x;
            while self.try_move(board, vec2(1, 0)) {
                test(self, board, rotation);
            }
            self.pos.x = pos.x;
            if !self.try_turn(board, false) {
                for _ in 0..rotation {
                    self.turn(true);
                }
                break;
            }
        }

        if best.1 > 0 {
            self.try_turn(board, best.1 > 2);
        } else if best.0 != self.pos.x {
            self.try_move(board, vec2((best.0 - self.pos.x).signum() as _, 0));
        } else {
            return best.2 < board.size.y - 3;
        }
        false
    }
}
