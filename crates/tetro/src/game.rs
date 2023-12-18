use super::tetromino::Tetromino;
use batbox_la::*;
use bidivec::BidiVec;
use rand::Rng;
use std::time::{Duration, Instant};
use tween::Tweener;

#[derive(Clone, Debug)]
pub struct Board {
    pub size: vec2<usize>,
    pub field: BidiVec<Option<(f64, f64, f64)>>,
    pub zone_lines: Vec<Tweener<f64, f64, tween::CubicInOut>>,
}

impl Board {
    pub fn new(size: vec2<usize>) -> Self {
        Self {
            size,
            field: BidiVec::with_elem(None, size.x, size.y),
            zone_lines: Vec::new(),
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
        let origin = origin.unwrap_or(self.size.y - self.zone_lines.len() - 1);
        match offset.cmp(&0) {
            std::cmp::Ordering::Less => {
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
            }
            std::cmp::Ordering::Greater => {
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
            std::cmp::Ordering::Equal => (),
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

    pub fn draw(
        &mut self,
        context: &cairo::Context,
        tile: f64,
        offset: vec2<f64>,
        frame_time: f64,
    ) {
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

        context.set_source_rgb(1.0, 1.0, 1.0);
        for y in &mut self.zone_lines {
            context.rectangle(
                offset.x,
                offset.y + y.move_by(frame_time) * tile,
                self.size.x as f64 * tile,
                tile,
            );
            context.fill().unwrap();
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Normal,
    Zone,
    ZoneEnding,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub board: Board,
    pub tetromino: Tetromino,
    pub placed: bool,

    pub particles: Vec<Particle>,

    pub timer: Instant,
    pub move_time: Duration,
    pub general_move_time: Duration,

    pub state: State,
    pub zone_meter: f64,
    pub zone_max: f64,

    pub last_frame: Instant,
}

#[derive(Clone, Debug)]
pub struct Particle {
    position: vec2<f64>,
    velocity: vec2<Tweener<f64, f64, tween::SineIn>>,
    size: Tweener<f64, f64, tween::CubicIn>,
    color_rg: Tweener<f64, f64, tween::CubicIn>,
    angle: f64,
}

impl Particle {
    pub fn new(position: vec2<f64>) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            position,
            velocity: vec2(
                Tweener::sine_in(0.0, rng.gen_range(-20.0..20.0), 1.0),
                Tweener::sine_in(0.0, rng.gen_range(-20.0..20.0), 1.0),
            ),
            size: Tweener::cubic_in(5.0, 0.0, 2.0),
            color_rg: Tweener::cubic_in(1.0, 0.0, 2.0),
            angle: 0.0,
        }
    }

    pub fn frame(&mut self, context: &cairo::Context, offset: vec2<f64>, frame_time: f64) {
        let velocity = vec2(
            self.velocity.x.move_by(frame_time),
            self.velocity.y.move_by(frame_time),
        );
        self.position += velocity * frame_time;
        let rg = self.color_rg.move_by(frame_time);
        let size = self.size.move_by(frame_time);
        if !self.size.is_finished() {
            let center = offset + self.position;
            context.set_source_rgb(rg, rg, 1.0);
            context.translate(center.x, center.y);
            context.rotate(self.angle);
            context.rectangle(-size * 0.5, -size * 0.5, size, size);
            context.fill().unwrap();
            context.identity_matrix();
        }
    }
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

            particles: Vec::new(),

            state: State::Normal,
            zone_meter: 0.0,
            zone_max: 20.0,

            last_frame: std::time::Instant::now(),
        }
    }

    fn particle_rect(&mut self, tl: vec2<f64>, size: vec2<f64>) {
        for y in (0..=size.y as usize).step_by(5) {
            for x in (0..=size.x as usize).step_by(5) {
                self.particles
                    .push(Particle::new(tl + vec2(x as _, y as _)));
            }
        }
    }

    pub fn frame(
        &mut self,
        context: &cairo::Context,
        tile: f64,
        offset: vec2<f64>,
        opponent: Option<&mut Game>,
    ) -> bool {
        let frame_time = self.last_frame.elapsed().as_secs_f64();
        self.last_frame = std::time::Instant::now();

        if self.timer.elapsed() >= self.move_time && self.state == State::Normal {
            self.timer = std::time::Instant::now();
            if !self.tetromino.try_move(&self.board, vec2(0, 1)) {
                self.tetromino.place(&mut self.board);
                self.placed = true;
            }
        }
        if self.placed {
            self.placed = false;
            self.tetromino = Tetromino::random(self.board.size.x / 2);
            if !self.tetromino.fits(&self.board) {
                return false;
            }
        }

        {
            let cleared_lines = self.board.full_lines().collect::<Vec<_>>();
            for &y in &cleared_lines {
                self.board.shift(Some(y), 1, || None);
                if self.state == State::Normal {
                    let tl = vec2(0.0, y as f64 * tile);
                    let size = vec2(self.board.size.x as f64 * tile, tile);
                    self.particle_rect(tl, size);
                }
            }

            if self.state == State::Zone {
                self.zone_meter -= frame_time;
                if self.zone_meter <= 0.0 {
                    self.zone_meter = 0.0;
                    self.state = State::ZoneEnding;
                }

                self.board
                    .shift(None, -(cleared_lines.len() as isize), || None);
                for &line in cleared_lines.iter().rev() {
                    self.board.zone_lines.push(Tweener::cubic_in_out(
                        line as f64,
                        (self.board.size.y - self.board.zone_lines.len() - 1) as f64,
                        1.0,
                    ));
                }
            } else if self.state == State::Normal {
                self.zone_meter += cleared_lines.len() as f64 / 2.0;
                self.zone_meter = self.zone_meter.min(self.zone_max);
            }
        }

        if self.state == State::ZoneEnding {
            let mut zone_finished = true;
            for line in &self.board.zone_lines {
                if !line.is_finished() {
                    zone_finished = false;
                }
            }
            if zone_finished {
                self.state = State::Normal;
                if let Some(opponent) = opponent {
                    opponent.board.garbage(self.board.zone_lines.len() / 4);
                    // if self.board.zone_lines.len() >= 8 {
                    //     opponent.tetromino = opponent.tetromino.clone().scale(2);
                    // }
                }
                let zone_lines = self.board.zone_lines.len();
                {
                    let tl = vec2(
                        0.0,
                        (self.board.size.y - self.board.zone_lines.len()) as f64 * tile,
                    );
                    let size = vec2(
                        self.board.size.x as f64 * tile,
                        self.board.zone_lines.len() as f64 * tile,
                    );
                    self.particle_rect(tl, size);
                }
                self.board.zone_lines.clear();
                self.board.shift(None, zone_lines as _, || None);
            }
        }

        self.board.draw(context, tile, offset, frame_time);
        self.tetromino.draw(context, tile, offset);
        let mut shadow = self.tetromino.clone();
        shadow.drop(&self.board);
        shadow.draw_shadow(context, tile, offset);

        for particle in &mut self.particles {
            particle.frame(context, offset, frame_time);
        }
        self.particles
            .retain(|particle| particle.size.clone().move_by(0.0) > 0.0);

        let zone_pos = offset + vec2(-2.1, 1.2) * tile;
        context.set_source_rgb(0.0, 0.2, 1.0);
        context.set_line_width(1.0);
        context.arc(
            zone_pos.x,
            zone_pos.y,
            tile + 3.5,
            0.0,
            std::f64::consts::PI * 2.0,
        );
        context.stroke().unwrap();
        context.arc(
            zone_pos.x,
            zone_pos.y,
            tile - 3.5,
            0.0,
            std::f64::consts::PI * 2.0,
        );
        context.stroke().unwrap();

        context.set_line_width(5.0);
        context.arc(
            zone_pos.x,
            zone_pos.y,
            tile,
            -std::f64::consts::PI / 2.0,
            self.zone_meter / self.zone_max * std::f64::consts::PI * 2.0
                - std::f64::consts::PI / 2.0,
        );
        context.stroke().unwrap();

        true
    }

    pub fn try_move(&mut self, direction: i8) {
        self.tetromino.try_move(&self.board, vec2(direction, 0));
    }

    pub fn try_turn(&mut self, ccw: bool) {
        self.tetromino.try_turn(&self.board, ccw);
    }

    pub fn zone(&mut self) {
        self.state = State::Zone;
    }

    pub fn speedup(&mut self, speedup: bool) {
        if speedup {
            if self.state == State::Zone {
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
