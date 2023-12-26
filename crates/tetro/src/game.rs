use super::tetromino::Tetromino;
use batbox_la::*;
use bidivec::BidiVec;
use rand::Rng;
use scheduler::*;
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
        let ____________________board____________________ = std::time::Instant::now();
        let ____________________grid____________________ = std::time::Instant::now();
        context.set_source_rgb(0.0, 0.2, 1.0);
        context.set_line_width(4.0);
        context.rectangle(
            offset.x,
            offset.y,
            self.size.x as f64 * tile,
            self.size.y as f64 * tile,
        );
        log_error!("{}"; context.stroke());

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
            }
        }
        log_error!("{}"; context.stroke());
        println!(
            "Rendering grid took {}ms",
            ____________________grid____________________
                .elapsed()
                .as_millis()
        );

        let ____________________blocks____________________ = std::time::Instant::now();
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
                    log_error!("{}"; context.fill());
                }
            }
        }
        println!(
            "Rendering blocks took {}ms",
            ____________________blocks____________________
                .elapsed()
                .as_millis()
        );

        let ____________________zone_lines____________________ = std::time::Instant::now();
        context.set_source_rgb(1.0, 1.0, 1.0);
        for y in &mut self.zone_lines {
            context.rectangle(
                offset.x,
                offset.y + y.move_by(frame_time).floor() * tile,
                self.size.x as f64 * tile,
                tile + 2.0,
            );
            log_error!("{}"; context.fill());
        }
        println!(
            "Rendering zone lines took {}ms",
            ____________________zone_lines____________________
                .elapsed()
                .as_millis()
        );
        println!(
            "Rendering board took {}ms",
            ____________________board____________________
                .elapsed()
                .as_millis()
        );
    }
}

#[derive(Clone, Debug)]
pub struct Particle {
    position: vec2<f64>,
    velocity: vec2<Tweener<f64, f64, tween::SineIn>>,
    size: Tweener<f64, f64, tween::CubicIn>,
    color_rga: Tweener<f64, f64, tween::CubicIn>,
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
            size: Tweener::cubic_in(20.0, 0.0, 2.0),
            color_rga: Tweener::cubic_in(1.0, 0.0, 2.0),
            angle: 0.0,
        }
    }

    pub fn frame(&mut self, context: &cairo::Context, offset: vec2<f64>, frame_time: f64) {
        let velocity = vec2(
            self.velocity.x.move_by(frame_time),
            self.velocity.y.move_by(frame_time),
        );
        self.position += velocity * frame_time;
        let rga = self.color_rga.move_by(frame_time);
        let size = self.size.move_by(frame_time);
        if !self.size.is_finished() {
            let center = offset + self.position;
            context.set_source_rgba(rga, rga, 1.0, rga);
            context.translate(center.x, center.y);
            context.rotate(self.angle);
            context.rectangle(
                -(size * 0.5).floor(),
                -(size * 0.5).floor(),
                size.floor(),
                size.floor(),
            );
            log_error!("{}"; context.fill());
            context.identity_matrix();
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Normal,
    Zone,
    ZoneEnding,
    GameOver,
    Won,
}

#[derive(Debug)]
pub struct Game {
    pub uid: String,
    pub name: String,

    pub board: Board,
    pub tetromino: Tetromino,
    pub placed: bool,

    pub particles: Vec<Particle>,

    pub timer: Instant,
    pub move_time: Duration,
    pub general_move_time: Duration,

    pub points: u64,
    pub zone_meter: f64,
    pub zone_max: f64,
    pub state: State,

    pub last_frame: Instant,

    pub block_fall: soloud::audio::Wav,
    pub line_clear: soloud::audio::Wav,
    pub zone_line_fall: soloud::audio::Wav,
    pub zone_finish: soloud::audio::Wav,
}

impl Game {
    pub fn new(size: vec2<usize>, uid: String, name: String) -> Self {
        Self {
            uid,
            name,

            board: Board::new(size),
            tetromino: Tetromino::random(size.x / 2),
            placed: false,

            timer: Instant::now(),
            move_time: Duration::from_millis(500),
            general_move_time: Duration::from_millis(500),

            particles: Vec::new(),

            points: 0,
            zone_meter: 0.0,
            zone_max: 20.0,
            state: State::Normal,

            last_frame: std::time::Instant::now(),

            block_fall: load_wav("Assets/tetro/block-fall.wav"),
            line_clear: load_wav("Assets/tetro/line-clear.wav"),
            zone_line_fall: load_wav("Assets/tetro/zone-line-fall.wav"),
            zone_finish: load_wav("Assets/tetro/zone-finish.wav"),
        }
    }

    pub fn add_points(&mut self, amount: u64) {
        self.points += amount;
        if self.uid != "AI" {
            let uid = self.uid.clone();
            spawn_in_server_runtime(async move {
                points::give(&uid, amount).await;
            });
        }
    }

    pub fn lines_cleared_points(lines: usize) -> u64 {
        (lines as f32).powf(1.5) as _
    }

    fn particle_rect(&mut self, tl: vec2<f64>, size: vec2<f64>) {
        for y in (10..=size.y as usize).step_by(10) {
            for x in (10..=size.x as usize).step_by(10) {
                self.particles
                    .push(Particle::new(tl + vec2(x as _, y as _)));
            }
        }
    }

    pub fn draw(
        &mut self,
        context: &cairo::Context,
        tile: f64,
        offset: vec2<f64>,
        frame_time: f64,
    ) {
        let offset = offset + vec2(0.0, tile * 1.5);
        self.board.draw(context, tile, offset, frame_time);
        if self.state == State::GameOver {
            self.zone_meter -= self.zone_meter * 0.1;
        } else if self.state == State::Won {
            self.zone_meter += (self.zone_max - self.zone_meter) * 0.1;
        } else {
            self.tetromino.draw(context, tile, offset);
            let mut shadow = self.tetromino.clone();
            shadow.drop(&self.board);
            shadow.draw_shadow(context, tile, offset);
        }

        for particle in &mut self.particles {
            particle.frame(context, offset, frame_time);
        }
        self.particles
            .retain(|particle| particle.size.clone().move_by(0.0) > 0.0);

        let zone_pos = offset + (vec2(-2.1, 1.2) * tile).map(f64::floor);
        context.set_source_rgb(0.0, 0.2, 1.0);
        context.set_line_width(1.0);
        context.arc(
            zone_pos.x,
            zone_pos.y,
            tile + 3.5,
            0.0,
            std::f64::consts::PI * 2.0,
        );
        log_error!("{}"; context.stroke());
        context.arc(
            zone_pos.x,
            zone_pos.y,
            tile - 3.5,
            0.0,
            std::f64::consts::PI * 2.0,
        );
        log_error!("{}"; context.stroke());

        context.set_line_width(6.0);
        context.arc(
            zone_pos.x,
            zone_pos.y,
            tile,
            -std::f64::consts::PI / 2.0,
            self.zone_meter / self.zone_max * std::f64::consts::PI * 2.0
                - std::f64::consts::PI / 2.0,
        );
        log_error!("{}"; context.stroke());

        context.set_source_rgb(1.0, 1.0, 1.0);
        context.set_font_size(tile);

        if let Some(text_offset) = text_center_offset(context, &self.name) {
            context.move_to(
                offset.x + (self.board.size.x as f64 * tile) / 2.0 - text_offset.x,
                offset.y - tile * 0.5,
            );
            context.show_text(&self.name).ok();
        }

        if let Some(text) = match self.state {
            State::GameOver => Some("Lost"),
            State::Won => Some("Won"),
            _ => None,
        } {
            if let Some(text_offset) = text_center_offset(context, text) {
                context.move_to(
                    offset.x + (self.board.size.x as f64 * tile) / 2.0 - text_offset.x,
                    offset.y + tile * 2.5,
                );
                context.show_text(text).ok();
            }
        }

        let points = self.points.to_string();
        if let Some(text_offset) = text_center_offset(context, &points) {
            context.move_to(zone_pos.x - text_offset.x, zone_pos.y + tile * 2.5);
            context.show_text(&points).ok();
        }
    }

    pub fn update(
        &mut self,
        soloud: &soloud::Soloud,
        tile: f64,
        frame_time: f64,
        opponent: Option<&mut Game>,
    ) -> bool {
        if self.uid == "AI" {
            if self.tetromino.ai(&mut self.board) {
                self.speedup(true);
            }
            if self.zone_meter >= self.zone_max * 0.4 {
                self.zone();
            }
        }

        if self.timer.elapsed() >= self.move_time && self.state == State::Normal {
            self.timer = std::time::Instant::now();
            if !self.tetromino.try_move(&self.board, vec2(0, 1)) {
                self.tetromino.place(&mut self.board);
                self.placed = true;
            }
        }
        if self.placed {
            self.placed = false;
            soloud.play(&self.block_fall);
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
                    let min_move_time = Duration::from_millis(20);
                    self.general_move_time = (self.general_move_time * 70 / 100).max(min_move_time);
                    self.move_time = self.move_time.min(self.general_move_time);

                    let tl = vec2(0.0, y as f64 * tile);
                    let size = vec2(self.board.size.x as f64 * tile, tile);
                    self.particle_rect(tl, size);
                    soloud.play(&self.line_clear);
                } else {
                    soloud.play(&self.zone_line_fall);
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
            } else if self.state == State::Normal && !cleared_lines.is_empty() {
                self.add_points(Self::lines_cleared_points(cleared_lines.len()));
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
                soloud.play(&self.zone_finish);
                self.state = State::Normal;
                if let Some(opponent) = opponent {
                    opponent
                        .board
                        .garbage(((self.board.zone_lines.len() / 4) as f32).powf(1.5) as _);
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
                self.add_points(Self::lines_cleared_points(zone_lines));

                self.board.zone_lines.clear();
                self.board.shift(None, zone_lines as _, || None);
            }
        }

        true
    }

    pub fn build_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&(self.board.size.x as u32).to_le_bytes());
        message.extend_from_slice(&(self.board.size.y as u32).to_le_bytes());
        for tile in self.board.field.iter() {
            message.extend_from_slice(&tile.map_or(0, color_to_u32).to_le_bytes());
        }
        message.extend_from_slice(&self.points.to_le_bytes());
        message.extend_from_slice(&self.zone_meter.to_le_bytes());
        message.extend_from_slice(&self.zone_max.to_le_bytes());
        message.push((self.state == self::State::Zone) as u8);
        message.extend_from_slice(&(self.board.zone_lines.len() as u32).to_le_bytes());
        for line in &self.board.zone_lines {
            message.extend_from_slice(&line.clone().move_by(0.0).to_le_bytes());
        }

        message.extend_from_slice(&self.tetromino.pos.x.to_le_bytes());
        message.extend_from_slice(&self.tetromino.pos.y.to_le_bytes());
        message.extend_from_slice(&(self.tetromino.size as u32).to_le_bytes());
        message.extend_from_slice(&color_to_u32(self.tetromino.color).to_le_bytes());
        message.extend_from_slice(&(self.tetromino.blocks.len() as u32).to_le_bytes());
        for block in &self.tetromino.blocks {
            message.push(block.x);
            message.push(block.y);
        }

        message
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
            self.move_time = Duration::from_millis(50).min(self.general_move_time);
        } else {
            self.move_time = self.general_move_time;
        }
    }

    pub fn game_over(&mut self, tile: f64) {
        self.state = State::GameOver;
        self.explode(tile);
    }

    pub fn won(&mut self, tile: f64) {
        self.add_points(self.points);
        self.state = State::Won;
        self.explode(tile);
    }

    pub fn explode(&mut self, tile: f64) {
        self.board.zone_lines.clear();
        for y in 0..self.board.size.y {
            for x in 0..self.board.size.x {
                self.particle_rect(vec2(x, y).map(|x| x as _) * tile, vec2::splat(tile));
                self.board.set(vec2(x, y), None);
            }
        }
    }
}
