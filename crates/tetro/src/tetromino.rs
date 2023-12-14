use super::game::*;
use batbox_la::*;
use rand::Rng;

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
            let last = block.clone();
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
        if attempt.fit(board) {
            *self = attempt;
            true
        } else {
            false
        }
    }

    pub fn try_move(&mut self, board: &Board, direction: vec2<i8>) -> bool {
        self.pos += direction.map(i32::from);
        if !self.fit(board) {
            self.pos -= direction.map(i32::from);
            false
        } else {
            true
        }
    }

    pub fn drop(&mut self, board: &Board) {
        while self.try_move(board, vec2::UNIT_Y) {}
    }

    pub fn place(&self, board: &mut Board) {
        for block in self.blocks() {
            if Aabb2::from_corners(vec2::ZERO, board.size.map(|x| x as _)).contains(block) {
                board.set(block.map(|x| x as _), Some(self.color));
            }
        }
    }

    pub fn fit(&self, board: &Board) -> bool {
        for block in self.blocks() {
            if !Aabb2::from_corners(vec2::ZERO, board.size.map(|x| x as _)).contains(block)
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
    pub fn ai(&mut self, board: &mut Board) {
        let pos = self.pos.clone();
        let mut best = (self.pos.x, 0, 0.0);
        let mut test = |tetromino: &mut Self, board: &mut Board, rotation| {
            if !tetromino.fit(board) {
                return;
            }
            tetromino.drop(board);
            tetromino.place(board);

            let mut score = 0.0;
            score += tetromino.blocks().map(|block| block.y as f32).sum::<f32>() * 0.5;
            score += tetromino.blocks().map(|block| block.y).max().unwrap_or(0) as f32;
            score += board.full_lines().count() as f32 * 3.0;
            score -= tetromino
                .blocks()
                .filter(|block| {
                    board
                        .get(block.map(|x| x as usize) + vec2::UNIT_Y)
                        .is_none()
                })
                .count() as f32;

            if score > best.2 {
                best = (tetromino.pos.x, rotation, score);
            }

            for block in tetromino.blocks() {
                board.set(block.map(|x| x as _), None);
            }
            tetromino.pos.y = pos.y;
        };

        for rotation in 0..4 {
            test(self, board, rotation);
            self.try_turn(board, false);
        }
        while self.try_move(board, vec2(-1, 0)) {
            for rotation in 0..4 {
                test(self, board, rotation);
                self.turn(false);
            }
        }
        self.pos.x = pos.x;
        while self.try_move(board, vec2(1, 0)) {
            for rotation in 0..4 {
                test(self, board, rotation);
                self.turn(false);
            }
        }
        self.pos = pos;

        if best.0 != self.pos.x {
            self.try_move(board, vec2((best.0 - self.pos.x).signum() as _, 0));
        } else if best.1 > 0 {
            self.try_turn(board, best.1 > 2);
        } else {
            // self.drop(board);
            // self.place(board);
        }
    }
}
