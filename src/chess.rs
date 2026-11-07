//! Legal chess and incremental Zobrist hashing. a1 = 0, h8 = 63.
use std::fmt;
pub const PAWN: i8 = 1;
pub const KNIGHT: i8 = 2;
pub const BISHOP: i8 = 3;
pub const ROOK: i8 = 4;
pub const QUEEN: i8 = 5;
pub const KING: i8 = 6;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: usize,
    pub to: usize,
    pub promotion: i8,
}
impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            square_name(self.from),
            square_name(self.to),
            match self.promotion {
                5 => "q",
                4 => "r",
                3 => "b",
                2 => "n",
                _ => "",
            }
        )
    }
}
pub fn square_name(s: usize) -> String {
    format!("{}{}", (b'a' + (s % 8) as u8) as char, s / 8 + 1)
}
/// SplitMix64 provides deterministic independent pseudorandom Zobrist features.
pub fn random_key(mut n: u64) -> u64 {
    n = n.wrapping_add(0x9e3779b97f4a7c15);
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}
fn piece_key(p: i8, s: usize) -> u64 {
    random_key(0x4348455353 + (p + 6) as u64 * 64 + s as u64)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub squares: [i8; 64],
    pub white: bool,
    pub castling: u8,
    pub ep: Option<usize>,
    pub halfmove: u16,
    pub fullmove: u16,
    pub hash: u64,
}
impl Default for Position {
    fn default() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }
}
impl Position {
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let f: Vec<_> = fen.split_whitespace().collect();
        if f.len() != 6 {
            return Err("FEN needs six fields".into());
        }
        let mut p = Self {
            squares: [0; 64],
            white: f[1] == "w",
            castling: 0,
            ep: None,
            halfmove: f[4].parse().map_err(|_| "Invalid clock")?,
            fullmove: f[5].parse().map_err(|_| "Invalid move number")?,
            hash: 0,
        };
        if f[1] != "w" && f[1] != "b" {
            return Err("Invalid side".into());
        }
        let ranks: Vec<_> = f[0].split('/').collect();
        if ranks.len() != 8 {
            return Err("Invalid ranks".into());
        }
        for (row, rank) in ranks.iter().enumerate() {
            let mut x = 0;
            for ch in rank.chars() {
                if let Some(n) = ch.to_digit(10) {
                    if n == 0 || n > 8 {
                        return Err("Invalid count".into());
                    }
                    x += n as usize;
                } else {
                    let pc = match ch.to_ascii_lowercase() {
                        'p' => 1,
                        'n' => 2,
                        'b' => 3,
                        'r' => 4,
                        'q' => 5,
                        'k' => 6,
                        _ => return Err("Invalid piece".into()),
                    };
                    if x >= 8 {
                        return Err("Rank overflow".into());
                    }
                    p.squares[(7 - row) * 8 + x] = if ch.is_uppercase() { pc } else { -pc };
                    x += 1;
                }
            }
            if x != 8 {
                return Err("Invalid width".into());
            }
        }
        for c in f[2].chars() {
            p.castling |= match c {
                'K' => 1,
                'Q' => 2,
                'k' => 4,
                'q' => 8,
                '-' => 0,
                _ => return Err("Invalid castling".into()),
            };
        }
        if f[3] != "-" {
            let s = f[3].as_bytes();
            if s.len() != 2 || !(b'a'..=b'h').contains(&s[0]) || !b"36".contains(&s[1]) {
                return Err("Invalid en passant".into());
            }
            p.ep = Some((s[1] - b'1') as usize * 8 + (s[0] - b'a') as usize);
        }
        if p.squares.iter().filter(|&&x| x == 6).count() != 1
            || p.squares.iter().filter(|&&x| x == -6).count() != 1
        {
            return Err("Each side needs one king".into());
        }
        p.hash = p.compute_hash();
        Ok(p)
    }
    fn metadata_key(&self) -> u64 {
        random_key(10000 + self.castling as u64)
            ^ if self.white { 0 } else { random_key(11000) }
            ^ self.ep.map_or(0, |s| random_key(12000 + s as u64))
    }
    pub fn compute_hash(&self) -> u64 {
        self.squares
            .iter()
            .enumerate()
            .filter(|(_, p)| **p != 0)
            .fold(self.metadata_key(), |h, (s, &p)| h ^ piece_key(p, s))
    }
    fn set(&mut self, s: usize, p: i8) {
        if self.squares[s] != 0 {
            self.hash ^= piece_key(self.squares[s], s);
        }
        self.squares[s] = p;
        if p != 0 {
            self.hash ^= piece_key(p, s);
        }
    }
    pub fn is_capture(&self, m: Move) -> bool {
        self.squares[m.to] != 0
            || (self.squares[m.from].abs() == 1 && self.ep == Some(m.to) && m.from % 8 != m.to % 8)
    }
    /// Apply a move already validated by legal_moves.
    pub fn play(&self, m: Move) -> Self {
        let mut p = self.clone();
        p.hash ^= p.metadata_key();
        let piece = p.squares[m.from];
        let capture = self.is_capture(m);
        p.set(m.from, 0);
        if piece.abs() == 1
            && self.ep == Some(m.to)
            && self.squares[m.to] == 0
            && m.from % 8 != m.to % 8
        {
            p.set(if self.white { m.to - 8 } else { m.to + 8 }, 0);
        }
        p.set(
            m.to,
            if m.promotion != 0 {
                m.promotion * piece.signum()
            } else {
                piece
            },
        );
        if piece.abs() == 6 {
            p.castling &= if self.white { 12 } else { 3 };
            if (m.from as i32 - m.to as i32).abs() == 2 {
                let (a, b) = if m.to > m.from {
                    (m.from + 3, m.from + 1)
                } else {
                    (m.from - 4, m.from - 1)
                };
                let rook = p.squares[a];
                p.set(a, 0);
                p.set(b, rook);
            }
        }
        for s in [m.from, m.to].iter() {
            p.castling &= match s {
                0 => !2,
                7 => !1,
                56 => !8,
                63 => !4,
                _ => 15,
            };
        }
        p.ep = if piece.abs() == 1 && (m.to as i32 - m.from as i32).abs() == 16 {
            Some((m.from + m.to) / 2)
        } else {
            None
        };
        p.halfmove = if piece.abs() == 1 || capture {
            0
        } else {
            self.halfmove.saturating_add(1)
        };
        if !self.white {
            p.fullmove = p.fullmove.saturating_add(1);
        }
        p.white = !self.white;
        p.hash ^= p.metadata_key();
        p
    }
    pub fn attacked(&self, s: usize, white: bool) -> bool {
        let sign = if white { 1 } else { -1 };
        let (x, y) = ((s % 8) as i32, (s / 8) as i32);
        for dx in [-1, 1].iter() {
            if let Some(t) = at(x + dx, y - if white { 1 } else { -1 }) {
                if self.squares[t] == sign {
                    return true;
                }
            }
        }
        for (dx, dy) in [
            (1, 2),
            (2, 1),
            (-1, 2),
            (-2, 1),
            (1, -2),
            (2, -1),
            (-1, -2),
            (-2, -1),
        ]
        .iter()
        {
            if let Some(t) = at(x + dx, y + dy) {
                if self.squares[t] == sign * 2 {
                    return true;
                }
            }
        }
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ]
        .iter()
        {
            let mut step = 1;
            while let Some(t) = at(x + dx * step, y + dy * step) {
                let pc = self.squares[t];
                if pc != 0 {
                    if pc.signum() == sign
                        && (pc.abs() == 5
                            || (step == 1 && pc.abs() == 6)
                            || (pc.abs() == 4 && (*dx == 0 || *dy == 0))
                            || (pc.abs() == 3 && *dx != 0 && *dy != 0))
                    {
                        return true;
                    }
                    break;
                }
                step += 1;
            }
        }
        false
    }
    pub fn in_check(&self, white: bool) -> bool {
        self.squares
            .iter()
            .position(|&p| p == if white { 6 } else { -6 })
            .map_or(true, |s| self.attacked(s, !white))
    }
    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(48);
        let sign = if self.white { 1 } else { -1 };
        for from in 0..64 {
            let pc = self.squares[from];
            if pc == 0 || pc.signum() != sign {
                continue;
            }
            let (x, y) = ((from % 8) as i32, (from / 8) as i32);
            let mut add = |to: usize| {
                if pc.abs() == 1 && (to / 8 == 0 || to / 8 == 7) {
                    for promotion in [5, 4, 3, 2].iter() {
                        moves.push(Move {
                            from,
                            to,
                            promotion: *promotion,
                        });
                    }
                } else {
                    moves.push(Move {
                        from,
                        to,
                        promotion: 0,
                    });
                }
            };
            match pc.abs() {
                1 => {
                    let dy = sign as i32;
                    if let Some(to) = at(x, y + dy) {
                        if self.squares[to] == 0 {
                            add(to);
                            if y == if self.white { 1 } else { 6 } {
                                let t = at(x, y + dy * 2).unwrap();
                                if self.squares[t] == 0 {
                                    add(t);
                                }
                            }
                        }
                    }
                    for dx in [-1, 1].iter() {
                        if let Some(to) = at(x + dx, y + dy) {
                            let target = self.squares[to];
                            let ep = self.ep == Some(to)
                                && target == 0
                                && self.squares[if self.white { to - 8 } else { to + 8 }] == -sign;
                            if (target.signum() == -sign && target.abs() != 6) || ep {
                                add(to);
                            }
                        }
                    }
                }
                2 => {
                    for (dx, dy) in [
                        (1, 2),
                        (2, 1),
                        (-1, 2),
                        (-2, 1),
                        (1, -2),
                        (2, -1),
                        (-1, -2),
                        (-2, -1),
                    ]
                    .iter()
                    {
                        if let Some(to) = at(x + dx, y + dy) {
                            if self.squares[to].signum() != sign && self.squares[to].abs() != 6 {
                                add(to);
                            }
                        }
                    }
                }
                _ => {
                    for (dx, dy) in [
                        (1, 0),
                        (-1, 0),
                        (0, 1),
                        (0, -1),
                        (1, 1),
                        (-1, 1),
                        (1, -1),
                        (-1, -1),
                    ]
                    .iter()
                    {
                        if pc.abs() == 3 && (*dx == 0 || *dy == 0) {
                            continue;
                        }
                        if pc.abs() == 4 && *dx != 0 && *dy != 0 {
                            continue;
                        }
                        let mut step = 1;
                        while let Some(to) = at(x + dx * step, y + dy * step) {
                            let t = self.squares[to];
                            if t.signum() == sign || t.abs() == 6 {
                                break;
                            }
                            add(to);
                            if t != 0 || pc.abs() == 6 {
                                break;
                            }
                            step += 1;
                        }
                    }
                    if pc.abs() == 6 && !self.in_check(self.white) {
                        let b = if self.white { 0 } else { 56 };
                        let rights = if self.white {
                            self.castling
                        } else {
                            self.castling >> 2
                        };
                        if from == b + 4 {
                            if rights & 1 != 0
                                && self.squares[b + 7] == sign * 4
                                && [5, 6].iter().all(|&s| {
                                    self.squares[b + s] == 0 && !self.attacked(b + s, !self.white)
                                })
                            {
                                add(b + 6);
                            }
                            if rights & 2 != 0
                                && self.squares[b] == sign * 4
                                && [1, 2, 3].iter().all(|&s| self.squares[b + s] == 0)
                                && [2, 3].iter().all(|&s| !self.attacked(b + s, !self.white))
                            {
                                add(b + 2);
                            }
                        }
                    }
                }
            }
        }
        moves
            .into_iter()
            .filter(|&m| !self.play(m).in_check(self.white))
            .collect()
    }
    /// En-passant availability matters for repetition only if a legal capture exists.
    pub fn repetition_key(&self) -> u64 {
        if let Some(ep) = self.ep {
            if !self
                .legal_moves()
                .iter()
                .any(|m| m.to == ep && self.squares[m.from].abs() == 1 && m.from % 8 != ep % 8)
            {
                return self.hash ^ random_key(12000 + ep as u64);
            }
        }
        self.hash
    }
    pub fn insufficient_material(&self) -> bool {
        let pcs: Vec<_> = self
            .squares
            .iter()
            .enumerate()
            .filter(|(_, p)| **p != 0 && p.abs() != 6)
            .collect();
        if pcs.is_empty() {
            return true;
        }
        if pcs.len() == 1 {
            return pcs[0].1.abs() == 3 || pcs[0].1.abs() == 2;
        }
        pcs.iter().all(|(_, p)| p.abs() == 3)
            && pcs
                .iter()
                .all(|(s, _)| (s % 8 + s / 8) % 2 == (pcs[0].0 % 8 + pcs[0].0 / 8) % 2)
    }
    /// Whether this side lacks mating material, allowing help from opposing
    /// pieces. This is asymmetric: a bare king cannot win on time even against
    /// a queen. Two knights can mate with cooperation and are not insufficient.
    /// Like the normal material draw check, this does not solve fortresses.
    pub fn insufficient_mating_material(&self, white: bool) -> bool {
        let own: Vec<_> = self
            .squares
            .iter()
            .enumerate()
            .filter(|(_, p)| **p != 0 && (**p > 0) == white && p.abs() != KING)
            .collect();
        if own.is_empty() {
            return true;
        }
        if own
            .iter()
            .any(|(_, p)| [PAWN, ROOK, QUEEN].contains(&p.abs()))
        {
            return false;
        }
        if own.iter().any(|(_, p)| p.abs() == KNIGHT) {
            if own.len() != 1 {
                return false;
            }
            // A lone knight can mate if an enemy pawn/minor/rook helps block
            // the enemy king; an enemy queen alone cannot provide that help.
            return !self.squares.iter().any(|&p| {
                p != 0 && (p > 0) != white && [PAWN, KNIGHT, BISHOP, ROOK].contains(&p.abs())
            });
        }
        // Only bishops remain. Opposite-color bishops, or any pawn/knight,
        // can make a cooperative mating position possible.
        let color = (own[0].0 % 8 + own[0].0 / 8) % 2;
        !self.squares.iter().enumerate().any(|(s, p)| {
            [PAWN, KNIGHT].contains(&p.abs()) || p.abs() == BISHOP && (s % 8 + s / 8) % 2 != color
        })
    }
    pub fn san(&self, m: Move, legal: &[Move]) -> String {
        let pc = self.squares[m.from].abs();
        let mut s = String::new();
        if pc == 6 && (m.from as i32 - m.to as i32).abs() == 2 {
            s.push_str(if m.to > m.from { "O-O" } else { "O-O-O" });
        } else {
            if pc != 1 {
                s.push(piece_letter(pc));
                let others: Vec<_> = legal
                    .iter()
                    .filter(|n| {
                        n.from != m.from && n.to == m.to && self.squares[n.from].abs() == pc
                    })
                    .collect();
                if !others.is_empty() {
                    if others.iter().all(|n| n.from % 8 != m.from % 8) {
                        s.push((b'a' + (m.from % 8) as u8) as char);
                    } else if others.iter().all(|n| n.from / 8 != m.from / 8) {
                        s.push((b'1' + (m.from / 8) as u8) as char);
                    } else {
                        s.push_str(&square_name(m.from));
                    }
                }
            } else if self.is_capture(m) {
                s.push((b'a' + (m.from % 8) as u8) as char);
            }
            if self.is_capture(m) {
                s.push('x');
            }
            s.push_str(&square_name(m.to));
            if m.promotion != 0 {
                s.push('=');
                s.push(piece_letter(m.promotion));
            }
        }
        let next = self.play(m);
        if next.in_check(next.white) {
            s.push(if next.legal_moves().is_empty() {
                '#'
            } else {
                '+'
            });
        }
        s
    }
}
pub fn piece_letter(p: i8) -> char {
    match p {
        1 => 'P',
        2 => 'N',
        3 => 'B',
        4 => 'R',
        5 => 'Q',
        6 => 'K',
        _ => '?',
    }
}
fn at(x: i32, y: i32) -> Option<usize> {
    if (0..8).contains(&x) && (0..8).contains(&y) {
        Some((y * 8 + x) as usize)
    } else {
        None
    }
}
