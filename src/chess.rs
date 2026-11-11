//! Legal chess and incremental Zobrist hashing. a1 = 0, h8 = 63.
use crate::tables::{KING_TARGETS, KNIGHT_TARGETS, PAWN_ATTACKERS, RAYS};
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
#[derive(Clone, Copy)]
pub(crate) struct MoveState {
    pub in_check: bool,
    pub has_legal: bool,
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
pub const fn random_key(mut n: u64) -> u64 {
    n = n.wrapping_add(0x9e3779b97f4a7c15);
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}
const fn piece_keys() -> [[u64; 64]; 13] {
    let mut keys = [[0; 64]; 13];
    let mut piece = 0;
    while piece < 13 {
        let mut square = 0;
        while square < 64 {
            keys[piece][square] = random_key(0x4348455353 + piece as u64 * 64 + square as u64);
            square += 1;
        }
        piece += 1;
    }
    keys
}
const fn metadata_keys<const N: usize>(base: u64) -> [u64; N] {
    let mut keys = [0; N];
    let mut index = 0;
    while index < N {
        keys[index] = random_key(base + index as u64);
        index += 1;
    }
    keys
}
static PIECE_KEYS: [[u64; 64]; 13] = piece_keys();
static CASTLING_KEYS: [u64; 16] = metadata_keys(10000);
static EP_KEYS: [u64; 64] = metadata_keys(12000);
const BLACK_TO_MOVE_KEY: u64 = random_key(11000);

fn piece_key(p: i8, s: usize) -> u64 {
    PIECE_KEYS[(p + 6) as usize][s]
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
        CASTLING_KEYS[self.castling as usize]
            ^ if self.white { 0 } else { BLACK_TO_MOVE_KEY }
            ^ self.ep.map_or(0, |s| EP_KEYS[s])
    }
    pub fn compute_hash(&self) -> u64 {
        self.squares
            .iter()
            .enumerate()
            .filter(|(_, p)| **p != 0)
            .fold(self.metadata_key(), |h, (s, &p)| h ^ piece_key(p, s))
    }
    fn set<const HASH: bool>(&mut self, s: usize, p: i8) {
        if HASH && self.squares[s] != 0 {
            self.hash ^= piece_key(self.squares[s], s);
        }
        self.squares[s] = p;
        if HASH && p != 0 {
            self.hash ^= piece_key(p, s);
        }
    }
    pub fn is_capture(&self, m: Move) -> bool {
        self.squares[m.to] != 0
            || (self.squares[m.from].abs() == 1 && self.ep == Some(m.to) && m.from % 8 != m.to % 8)
    }
    /// Apply a move already validated by legal_moves.
    pub fn play(&self, m: Move) -> Self {
        self.play_internal::<true>(m)
    }
    fn play_internal<const HASH: bool>(&self, m: Move) -> Self {
        let mut p = self.clone();
        let piece = p.squares[m.from];
        let capture = self.is_capture(m);
        p.set::<HASH>(m.from, 0);
        if piece.abs() == 1
            && self.ep == Some(m.to)
            && self.squares[m.to] == 0
            && m.from % 8 != m.to % 8
        {
            p.set::<HASH>(if self.white { m.to - 8 } else { m.to + 8 }, 0);
        }
        p.set::<HASH>(
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
                p.set::<HASH>(a, 0);
                p.set::<HASH>(b, rook);
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
        if HASH {
            p.hash ^= BLACK_TO_MOVE_KEY;
            if self.castling != p.castling {
                p.hash ^=
                    CASTLING_KEYS[self.castling as usize] ^ CASTLING_KEYS[p.castling as usize];
            }
            if let Some(ep) = self.ep {
                p.hash ^= EP_KEYS[ep];
            }
            if let Some(ep) = p.ep {
                p.hash ^= EP_KEYS[ep];
            }
        }
        p
    }
    fn leaper_attacked(&self, square: usize, white: bool) -> bool {
        let sign = if white { 1 } else { -1 };
        PAWN_ATTACKERS[usize::from(white)][square]
            .iter()
            .any(|s| self.squares[s] == sign)
            || KNIGHT_TARGETS[square]
                .iter()
                .any(|s| self.squares[s] == sign * KNIGHT)
            || KING_TARGETS[square]
                .iter()
                .any(|s| self.squares[s] == sign * KING)
    }
    pub fn attacked(&self, square: usize, white: bool) -> bool {
        if self.leaper_attacked(square, white) {
            return true;
        }
        let sign = if white { 1 } else { -1 };
        for (direction, ray) in RAYS[square].iter().enumerate() {
            for target in ray.iter() {
                let piece = self.squares[target];
                if piece != 0 {
                    if piece.signum() == sign
                        && (piece.abs() == QUEEN
                            || piece.abs() == ROOK && direction < 4
                            || piece.abs() == BISHOP && direction >= 4)
                    {
                        return true;
                    }
                    break;
                }
            }
        }
        false
    }
    /// Check status and absolute pins, computed once for the current king.
    fn king_safety(&self, king: usize) -> (bool, u64, u64) {
        let sign = if self.white { 1 } else { -1 };
        let mut check = self.leaper_attacked(king, !self.white);
        let mut evasions = u64::MAX;
        if check {
            for square in PAWN_ATTACKERS[usize::from(!self.white)][king].iter() {
                if self.squares[square] == -sign {
                    evasions &= 1u64 << square;
                }
            }
            for square in KNIGHT_TARGETS[king].iter() {
                if self.squares[square] == -sign * KNIGHT {
                    evasions &= 1u64 << square;
                }
            }
            for square in KING_TARGETS[king].iter() {
                if self.squares[square] == -sign * KING {
                    evasions &= 1u64 << square;
                }
            }
        }
        let mut pinned = 0;
        for (direction, ray) in RAYS[king].iter().enumerate() {
            let mut blocker = None;
            for target in ray.iter() {
                let piece = self.squares[target];
                if piece == 0 {
                    continue;
                }
                if piece.signum() == sign {
                    if blocker.is_some() {
                        break;
                    }
                    blocker = Some(target);
                } else {
                    if piece.abs() == QUEEN
                        || piece.abs() == ROOK && direction < 4
                        || piece.abs() == BISHOP && direction >= 4
                    {
                        if let Some(square) = blocker {
                            pinned |= 1u64 << square;
                        } else {
                            check = true;
                            // A non-king reply must capture this checker or
                            // block its ray. Intersect for multiple checks.
                            evasions &= ray
                                .iter()
                                .take_while(|&s| s != target)
                                .fold(1u64 << target, |mask, s| mask | (1u64 << s));
                        }
                    }
                    break;
                }
            }
        }
        (check, pinned, evasions)
    }
    pub fn in_check(&self, white: bool) -> bool {
        self.squares
            .iter()
            .position(|&p| p == if white { 6 } else { -6 })
            .map_or(true, |s| self.attacked(s, !white))
    }
    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(48);
        self.legal_moves_into(&mut moves);
        moves
    }
    pub(crate) fn legal_moves_into(&self, moves: &mut Vec<Move>) -> MoveState {
        self.generate_moves::<false, true>(moves)
    }
    pub(crate) fn tactical_moves_into(&self, moves: &mut Vec<Move>) -> MoveState {
        self.generate_moves::<true, true>(moves)
    }
    fn generate_moves<const TACTICAL: bool, const FAST: bool>(
        &self,
        moves: &mut Vec<Move>,
    ) -> MoveState {
        moves.clear();
        if moves.capacity() < 48 {
            moves.reserve(48);
        }
        let Some(king) = self
            .squares
            .iter()
            .position(|&p| p == if self.white { KING } else { -KING })
        else {
            return MoveState {
                in_check: true,
                has_legal: false,
            };
        };
        let (check, pinned, evasions) = if FAST {
            self.king_safety(king)
        } else {
            (self.in_check(self.white), 0, u64::MAX)
        };
        let mut has_legal = false;
        let sign = if self.white { 1 } else { -1 };
        for from in 0..64 {
            let pc = self.squares[from];
            if pc == 0 || pc.signum() != sign {
                continue;
            }
            let (x, y) = ((from % 8) as i32, (from / 8) as i32);
            let mut legal_seen = has_legal;
            let mut add = |to: usize| {
                let promotion = pc.abs() == PAWN && (to / 8 == 0 || to / 8 == 7);
                let en_passant = pc.abs() == PAWN
                    && self.ep == Some(to)
                    && self.squares[to] == 0
                    && from % 8 != to % 8;
                // En passant can remove a checker from another square or
                // change two blocking files, so retain full validation for it.
                if FAST && check && from != king && !en_passant && evasions & (1u64 << to) == 0 {
                    return has_legal;
                }
                let capture = self.squares[to] != 0 || en_passant;
                let collect = !TACTICAL || check || promotion || capture;
                if !collect && has_legal {
                    return has_legal;
                }
                let m = Move {
                    from,
                    to,
                    promotion: if promotion { QUEEN } else { 0 },
                };
                let legal = if FAST {
                    // Unpinned non-king moves cannot expose our king, except
                    // en passant, which removes a second blocking pawn.
                    !check && from != king && pinned & (1u64 << from) == 0 && !en_passant
                        || !self
                            .play_internal::<false>(m)
                            .attacked(if from == king { to } else { king }, !self.white)
                } else {
                    !self.play(m).in_check(self.white)
                };
                if !legal {
                    return has_legal;
                }
                has_legal = true;
                if collect {
                    if promotion {
                        for promotion in [QUEEN, ROOK, BISHOP, KNIGHT] {
                            moves.push(Move { promotion, ..m });
                        }
                    } else {
                        moves.push(m);
                    }
                }
                has_legal
            };
            match pc.abs() {
                1 => {
                    let dy = sign as i32;
                    if let Some(to) = at(x, y + dy) {
                        if self.squares[to] == 0 {
                            if !TACTICAL || check || !legal_seen || to / 8 == 0 || to / 8 == 7 {
                                legal_seen = add(to);
                            }
                            if y == if self.white { 1 } else { 6 } {
                                let t = at(x, y + dy * 2).unwrap();
                                if self.squares[t] == 0 && (!TACTICAL || check || !legal_seen) {
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
                    for to in KNIGHT_TARGETS[from].iter() {
                        if self.squares[to].signum() != sign
                            && self.squares[to].abs() != 6
                            && (!TACTICAL || check || !legal_seen || self.squares[to] != 0)
                        {
                            legal_seen = add(to);
                        }
                    }
                }
                _ => {
                    let directions = match pc.abs() {
                        BISHOP => 4..8,
                        ROOK => 0..4,
                        _ => 0..8,
                    };
                    for direction in directions {
                        for to in RAYS[from][direction].iter() {
                            let t = self.squares[to];
                            if t.signum() == sign || t.abs() == 6 {
                                break;
                            }
                            if TACTICAL && !check && legal_seen && t == 0 {
                                if pc.abs() == KING {
                                    break;
                                }
                                continue;
                            }
                            legal_seen = add(to);
                            if t != 0 || pc.abs() == 6 {
                                break;
                            }
                        }
                    }
                    if pc.abs() == 6 && !check && (!TACTICAL || !legal_seen) {
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
        MoveState {
            in_check: check,
            has_legal,
        }
    }
    /// En-passant availability matters for repetition only if a legal capture exists.
    pub fn repetition_key(&self) -> u64 {
        if let Some(ep) = self.ep {
            let sign = if self.white { 1 } else { -1 };
            let target = self.squares[ep];
            let captured = if self.white { ep - 8 } else { ep + 8 };
            let capture_possible = target.signum() == -sign && target.abs() != KING
                || target == 0 && self.squares[captured] == -sign;
            if capture_possible
                && PAWN_ATTACKERS[usize::from(self.white)][ep]
                    .iter()
                    .any(|from| self.squares[from] == sign)
            {
                if let Some(king) = self.squares.iter().position(|&p| p == sign * KING) {
                    for from in PAWN_ATTACKERS[usize::from(self.white)][ep].iter() {
                        if self.squares[from] == sign
                            && !self
                                .play_internal::<false>(Move {
                                    from,
                                    to: ep,
                                    promotion: 0,
                                })
                                .attacked(king, !self.white)
                        {
                            return self.hash;
                        }
                    }
                }
            }
            return self.hash ^ EP_KEYS[ep];
        }
        self.hash
    }
    pub fn insufficient_material(&self) -> bool {
        let (mut knights, mut bishops, mut bishop_colors) = (0, 0, 0u8);
        for (square, piece) in self.squares.iter().enumerate() {
            match piece.abs() {
                0 | KING => {}
                KNIGHT => knights += 1,
                BISHOP => {
                    bishops += 1;
                    bishop_colors |= 1 << ((square % 8 + square / 8) % 2);
                }
                _ => return false,
            }
        }
        knights + bishops <= 1 || knights == 0 && bishop_colors.count_ones() <= 1
    }
    /// Whether this side lacks mating material, allowing help from opposing
    /// pieces. This is asymmetric: a bare king cannot win on time even against
    /// a queen. Two knights can mate with cooperation and are not insufficient.
    /// Like the normal material draw check, this does not solve fortresses.
    pub fn insufficient_mating_material(&self, white: bool) -> bool {
        let (mut own_count, mut knights, mut color) = (0, 0, 0);
        for (square, &piece) in self.squares.iter().enumerate() {
            if piece == 0 || (piece > 0) != white || piece.abs() == KING {
                continue;
            }
            own_count += 1;
            if own_count == 1 {
                color = (square % 8 + square / 8) % 2;
            }
            if [PAWN, ROOK, QUEEN].contains(&piece.abs()) {
                return false;
            }
            knights += usize::from(piece.abs() == KNIGHT);
        }
        if own_count == 0 {
            return true;
        }
        if knights > 0 {
            if own_count != 1 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_hash_keys_match_the_original_splitmix_features() {
        for piece in -6..=6 {
            for square in 0..64 {
                assert_eq!(
                    piece_key(piece, square),
                    random_key(0x4348455353 + (piece + 6) as u64 * 64 + square as u64)
                );
            }
        }
        for (index, &key) in CASTLING_KEYS.iter().enumerate() {
            assert_eq!(key, random_key(10000 + index as u64));
        }
        for (index, &key) in EP_KEYS.iter().enumerate() {
            assert_eq!(key, random_key(12000 + index as u64));
        }
    }

    #[test]
    fn fast_and_tactical_generation_match_full_legality_over_playouts() {
        let seeds = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r3k2r/P6P/8/3pP3/8/8/p6p/R3K2R w KQkq d6 0 1",
            "k3r3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
            // Double check, en passant capture/block evasions, and promotions.
            "k3r3/8/8/8/1b6/8/2N5/4K3 w - - 0 1",
            "k7/8/8/3pP3/4K3/8/8/8 w - d6 0 1",
            "k7/2b5/8/2PpK3/8/8/8/8 w - d6 0 1",
            "1b2k3/P7/8/8/8/8/7K/8 w - - 0 1",
            "r6K/5P2/8/8/8/8/8/k7 w - - 0 1",
            "8/8/8/r4pPK/8/8/8/7k w - f6 0 1",
            "4k3/8/8/8/8/8/4r3/4K3 w - - 0 1",
            "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
            "7k/6Q1/6K1/8/8/8/8/8 b - - 0 1",
            "k7/8/8/8/1Q1Q1Q1Q/Q1Q1Q1Q1/8/7K w - - 0 1",
        ];
        let mut random = 0x51a7u64;
        for seed in seeds {
            for _ in 0..3 {
                let mut p = Position::from_fen(seed).unwrap();
                for _ in 0..200 {
                    let mut reference = Vec::new();
                    let slow = p.generate_moves::<false, false>(&mut reference);
                    let mut actual = Vec::new();
                    let state = p.legal_moves_into(&mut actual);
                    assert_eq!(actual, reference);
                    assert_eq!(state.in_check, p.in_check(p.white));
                    assert_eq!(state.has_legal, !reference.is_empty());
                    assert_eq!(state.in_check, slow.in_check);
                    let mut tactical = Vec::new();
                    let tactical_state = p.tactical_moves_into(&mut tactical);
                    let expected: Vec<_> = reference
                        .iter()
                        .copied()
                        .filter(|&m| state.in_check || p.is_capture(m) || m.promotion != 0)
                        .collect();
                    assert_eq!(tactical, expected);
                    assert_eq!(tactical_state.has_legal, state.has_legal);
                    assert_eq!(tactical_state.in_check, state.in_check);
                    let ep_matters = p.ep.is_some_and(|ep| {
                        reference.iter().any(|m| {
                            m.to == ep && p.squares[m.from].abs() == PAWN && m.from % 8 != ep % 8
                        })
                    });
                    assert_eq!(
                        p.repetition_key(),
                        p.hash
                            ^ p.ep
                                .filter(|_| !ep_matters)
                                .map_or(0, |ep| random_key(12000 + ep as u64))
                    );
                    assert_eq!(p.hash, p.compute_hash());
                    if reference.is_empty() {
                        break;
                    }
                    random = random_key(random);
                    p = p.play(reference[random as usize % reference.len()]);
                }
            }
        }
    }
}
