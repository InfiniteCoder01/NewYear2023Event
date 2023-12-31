use std::time::Instant;

use crate::*;
use batbox_la::*;
use bidivec::BidiVec;
use scheduler::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Cross,
    O,
    Triangle,
    Diamond,
}

impl Tag {
    pub fn no(index: usize) -> Self {
        match index % 4 {
            0 => Self::Cross,
            1 => Self::O,
            2 => Self::Triangle,
            3 => Self::Diamond,
            _ => unreachable!(),
        }
    }

    pub fn set_color(&self, context: &cairo::Context) {
        match self {
            Self::Cross => context.set_source_rgb(1.0, 0.0, 0.0),
            Self::O => context.set_source_rgb(0.0, 1.0, 0.0),
            Self::Triangle => context.set_source_rgb(0.0, 0.0, 1.0),
            Self::Diamond => context.set_source_rgb(1.0, 1.0, 0.0),
        }
    }

    pub fn draw(&self, context: &cairo::Context, offset: vec2<f64>, tile: f64, t: f64) {
        let htile = (tile / 2.0).floor();
        match self {
            Self::Cross => {
                let t1 = (t * 2.0).min(1.0);
                let t2 = (t * 2.0 - 1.0).max(0.0);
                context.move_to(offset.x, offset.y);
                context.line_to(offset.x + tile * t1, offset.y + tile * t1);
                context.move_to(offset.x + tile, offset.y);
                if t2 > 0.0 {
                    context.line_to(offset.x + tile * (1.0 - t2), offset.y + tile * t2);
                }
            }
            Self::O => {
                context.arc(
                    offset.x + htile,
                    offset.y + htile,
                    htile,
                    0.0,
                    2.0 * std::f64::consts::PI * t,
                );
            }
            Self::Triangle => {
                let t1 = (t * 3.0).min(1.0);
                let t2 = (t * 3.0 - 1.0).clamp(0.0, 1.0);
                let t3 = (t * 3.0 - 2.0).clamp(0.0, 1.0);
                context.move_to(offset.x, offset.y + tile);
                context.line_to(offset.x + tile * t1, offset.y + tile);
                if t2 > 0.0 {
                    context.line_to(
                        offset.x + htile + htile * (1.0 - t2),
                        offset.y + tile * (1.0 - t2),
                    );
                }
                if t3 > 0.0 {
                    context.line_to(offset.x + htile * (1.0 - t3), offset.y + tile * t3);
                }
            }
            Self::Diamond => {
                let t1 = (t * 4.0).min(1.0);
                let t2 = (t * 4.0 - 1.0).clamp(0.0, 1.0);
                let t3 = (t * 4.0 - 2.0).clamp(0.0, 1.0);
                let t4 = (t * 4.0 - 3.0).clamp(0.0, 1.0);
                context.move_to(offset.x + htile, offset.y);
                context.line_to(offset.x + htile + htile * t1, offset.y + htile * t1);
                if t2 > 0.0 {
                    context.line_to(
                        offset.x + htile + htile * (1.0 - t2),
                        offset.y + htile + htile * t2,
                    );
                }
                if t3 > 0.0 {
                    context.line_to(
                        offset.x + htile * (1.0 - t3),
                        offset.y + htile + htile * (1.0 - t3),
                    );
                }
                if t4 > 0.0 {
                    context.line_to(offset.x + htile * t4, offset.y + htile * (1.0 - t4));
                }
            }
        }
        log_error!("{}"; context.stroke());
    }

    fn encode(&self) -> u8 {
        match self {
            Tag::Cross => 1,
            Tag::O => 2,
            Tag::Triangle => 3,
            Tag::Diamond => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub uid: String,
    pub name: String,
    pub tag: Tag,
    pub score: usize,
}

impl Player {
    pub fn new(uid: String, name: String, tag: Tag) -> Self {
        Self {
            uid,
            name,
            tag,
            score: 0,
        }
    }
}

#[derive(Debug)]
pub struct Game {
    pub players: Vec<Player>,
    pub board: BidiVec<Option<(Tag, Instant)>>,
    pub turn: usize,
    pub turn_timeout: Instant,
    pub lines: Vec<(vec2<usize>, vec2<isize>, Instant)>,
}

impl Game {
    pub fn new(size: vec2<usize>, players: Vec<Player>) -> Self {
        Self {
            players,
            board: BidiVec::with_elem(None, size.x, size.y),
            turn: 0,
            turn_timeout: Instant::now(),
            lines: Vec::new(),
        }
    }

    pub fn draw(&mut self, context: &cairo::Context, tile: f64, offset: vec2<f64>) {
        let padding = 5.0;
        let htile = (tile / 2.0).floor();

        context.set_line_cap(cairo::LineCap::Round);
        context.set_source_rgb(1.0, 1.0, 1.0);
        context.set_line_width(3.0);
        context.rectangle(
            offset.x,
            offset.y,
            tile * self.board.width() as f64,
            tile * self.board.height() as f64,
        );
        log_error!("{}"; context.stroke());

        for y in 0..self.board.height() {
            for x in 0..self.board.width() {
                let offset = offset + vec2(x, y).map(|x| x as f64) * tile;
                context.set_source_rgb(1.0, 1.0, 1.0);
                context.rectangle(offset.x, offset.y, tile, tile);
                log_error!("{}"; context.stroke());

                if let Some(Some((tag, time))) = self.board.get(x, y) {
                    let offset = offset + vec2::splat(padding);
                    let tile = tile - padding * 2.0;
                    tag.set_color(context);
                    tag.draw(context, offset, tile, time.elapsed().as_secs_f64().min(1.0));
                }
            }
        }

        for (pos, dir, time) in &self.lines {
            let pos = pos.map(|x| x as f64);
            let dir = dir.map(|x| x as f64);
            let p1 = offset + pos * tile + vec2::splat(htile) - dir * htile + dir * padding;
            let p2 = p1 + dir * STRIDE as f64 * tile - dir * padding * 2.0;
            let p2 = p1 + (p2 - p1) * time.elapsed().as_secs_f64().min(1.0);
            context.set_source_rgb(1.0, 1.0, 1.0);
            context.move_to(p1.x, p1.y);
            context.line_to(p2.x, p2.y);
            log_error!("{}"; context.stroke());
        }

        let padding = 5.0;
        let mut offset = offset
            + vec2(
                self.board.width() as f64 * tile + padding,
                (self.board.height() as f64 * tile - self.players.len() as f64 * htile) / 2.0,
            )
            .map(f64::floor);

        context.set_font_size(htile - padding);
        for player in &self.players {
            let text = format!("{}: {}", player.name, player.score);
            player.tag.set_color(context);
            player.tag.draw(
                context,
                offset + vec2::splat(padding),
                htile - padding * 2.0,
                1.0,
            );
            if let Some(extents) = text_center_offset(context, &text) {
                context.move_to(
                    offset.x + htile + padding,
                    offset.y + (htile / 2.0 - extents.y).floor(),
                );
                context.set_source_rgb(1.0, 1.0, 1.0);
                log_error!("{}"; context.show_text(&text));
                log_error!("{}"; context.stroke());
            }
            offset.y += htile;
        }
    }

    pub fn update(&mut self, _soloud: &soloud::Soloud) -> bool {
        if self.players[self.turn].uid == "AI"
            && self.turn_timeout.elapsed() > std::time::Duration::from_secs(1)
        {
            let weights = [
                // [2, 1, 0, 1, 2],
                // [1, 4, 3, 4, 1],
                // [0, 3, 5, 3, 0],
                // [1, 4, 3, 4, 1],
                // [2, 1, 0, 1, 2],
                [2, 1, 0, 0, 1, 2],
                [1, 4, 3, 3, 4, 1],
                [0, 3, 5, 5, 3, 0],
                [0, 3, 5, 5, 3, 0],
                [1, 4, 3, 3, 4, 1],
                [2, 1, 0, 0, 1, 2],
            ];

            let mut possible_wins = Vec::new();
            for x in 0..self.board.width() {
                for y in 0..self.board.height() {
                    let pos = vec2(x, y);
                    for dir in [vec2(0, 1), vec2(1, -1), vec2(1, 0), vec2(1, 1)] {
                        if let Some(missing) = self.one_missing(pos, dir) {
                            possible_wins.push(missing);
                        }
                    }
                }
            }

            // if let Some((blocking_move, _)) = possible_wins
            //     .iter()
            //     .max_by_key(|&(_, tag)| tag == &self.players[self.turn].tag)
            // {
            //     self.try_turn(blocking_move.map(|x| x as _));
            // } else
            if let Some(best) = weights
                .iter()
                .enumerate()
                .flat_map(|(y, line)| {
                    line.iter()
                        .copied()
                        .enumerate()
                        .map(move |(x, score)| (vec2(x, y), score))
                })
                .filter(|&(pos, _)| self.board.get(pos.x, pos.y) == Some(&None))
                .rev()
                .max_by_key(|&(_, score)| score)
            {
                self.try_turn(best.0);
            }
        }

        if self.board.iter().all(|tile| tile.is_some()) {
            for player in &mut self.players {
                if player.uid != "AI" {
                    let uid = player.uid.clone();
                    let score = player.score;
                    spawn_in_server_runtime(async move {
                        points::give(&uid, score as _).await;
                    });
                }
            }

            false
        }else{true}
    }

    fn check(&self, pos: vec2<usize>, dir: vec2<isize>, stride: usize) -> Option<Tag> {
        if let Some((tag, _)) = self.board.get(pos.x, pos.y)? {
            for i in 1..stride {
                let pos = pos.map(|x| x as isize) + dir * (i as isize);
                if pos.x < 0
                    || pos.y < 0
                    || self.board.get(pos.x as _, pos.y as _)?.map(|cell| cell.0) != Some(*tag)
                {
                    return None;
                }
            }
            Some(*tag)
        } else {
            None
        }
    }

    fn one_missing(&self, pos: vec2<usize>, dir: vec2<isize>) -> Option<(vec2<usize>, Tag)> {
        if let Some(tag) = self.board.get(pos.x, pos.y)?.map(|cell| cell.0) {
            let mut missing = None;
            for i in 1..STRIDE {
                let pos = pos.map(|x| x as isize) + dir * (i as isize);
                if pos.x < 0 || pos.y < 0 {
                    return None;
                }
                match self.board.get(pos.x as _, pos.y as _)?.map(|cell| cell.0) {
                    Some(this_tag) if this_tag == tag => (),
                    None if missing.is_none() => missing = Some((pos.map(|x| x as _), tag)),
                    _ => return None,
                }
            }
            missing
        } else {
            let next = pos.map(|x| x as isize) + dir;
            if next.x < 0
                || next.y < 0
                || next.x >= self.board.width() as _
                || next.y >= self.board.height() as _
            {
                return None;
            }
            self.check(next.map(|x| x as _), dir, STRIDE - 1)
                .map(|tag| (pos, tag))
        }
    }

    pub fn build_message(&self, uid: &str) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&(self.board.width() as u32).to_le_bytes());
        message.extend_from_slice(&(self.board.height() as u32).to_le_bytes());
        for tile in self.board.iter() {
            message.push(tile.map_or(0, |(tag, _)| tag.encode()));
        }
        message.extend_from_slice(&(self.lines.len() as u32).to_le_bytes());
        for line in &self.lines {
            message.push(line.0.x as u8);
            message.push(line.0.y as u8);
            message.extend_from_slice(&(line.1.x as i8).to_le_bytes());
            message.extend_from_slice(&(line.1.y as i8).to_le_bytes());
        }

        if let Some(player) = self.players.iter().find(|player| player.uid == uid) {
            message.push(player.tag.encode());
            message.extend_from_slice(&(player.score as u32).to_le_bytes());
        }

        message
    }

    pub fn try_turn(&mut self, turn: vec2<usize>) {
        let tag = self.players[self.turn].tag;
        if let Some(tile) = self.board.get_mut(turn.x, turn.y) {
            if tile.is_some() {
                return;
            }
            *tile = Some((tag, Instant::now()));

            let mut rose = [0; 4];
            let directions = [
                vec2(-1, -1),
                vec2(-1, 0),
                vec2(-1, 1),
                vec2(0, 1),
                vec2(1, 1),
                vec2(1, 0),
                vec2(1, -1),
                vec2(0, -1),
            ];
            for (i, dir) in directions.into_iter().enumerate() {
                let mut depth = 0;
                while {
                    let pos = turn.map(|x| x as isize) + dir * (depth + 1);
                    pos.x >= 0
                        && pos.y >= 0
                        && self
                            .board
                            .get(pos.x as _, pos.y as _)
                            .map(|cell| cell.map(|cell| cell.0))
                            == Some(Some(tag))
                } {
                    depth += 1;
                }
                if i >= 4 {
                    if depth + rose[i - 4] >= 3 {
                        let dir = directions[i];
                        let pos = turn.map(|x| x as isize) - dir * rose[i - 4].min(3);
                        self.lines.push((pos.map(|x| x as _), dir, Instant::now()));
                        self.players[self.turn].score += 1;
                    }
                } else {
                    rose[i] = depth;
                }
            }

            self.skip_turn();
        }
    }

    pub fn skip_turn(&mut self) {
        self.turn = (self.turn + 1) % self.players.len();
        self.turn_timeout = Instant::now();
    }
}
