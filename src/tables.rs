//! Immutable board geometry, generated at compile time for every platform.

pub(crate) const DIRECTIONS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
];
const KNIGHT_STEPS: [(i32, i32); 8] = [
    (1, 2),
    (2, 1),
    (-1, 2),
    (-2, 1),
    (1, -2),
    (2, -1),
    (-1, -2),
    (-2, -1),
];

#[derive(Clone, Copy)]
pub(crate) struct Targets {
    squares: [u8; 8],
    len: u8,
}
impl Targets {
    const EMPTY: Self = Self {
        squares: [0; 8],
        len: 0,
    };
    pub(crate) fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.squares[..self.len as usize]
            .iter()
            .map(|&s| s as usize)
    }
}

const fn targets(steps: &[(i32, i32)]) -> [Targets; 64] {
    let mut result = [Targets::EMPTY; 64];
    let mut square = 0;
    while square < 64 {
        let (x, y) = ((square % 8) as i32, (square / 8) as i32);
        let mut index = 0;
        while index < steps.len() {
            let (nx, ny) = (x + steps[index].0, y + steps[index].1);
            if nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
                let len = result[square].len as usize;
                result[square].squares[len] = (ny * 8 + nx) as u8;
                result[square].len += 1;
            }
            index += 1;
        }
        square += 1;
    }
    result
}

const fn rays() -> [[Targets; 8]; 64] {
    let mut result = [[Targets::EMPTY; 8]; 64];
    let mut square = 0;
    while square < 64 {
        let mut direction = 0;
        while direction < 8 {
            let (dx, dy) = DIRECTIONS[direction];
            let (mut x, mut y) = ((square % 8) as i32 + dx, (square / 8) as i32 + dy);
            while x >= 0 && x < 8 && y >= 0 && y < 8 {
                let len = result[square][direction].len as usize;
                result[square][direction].squares[len] = (y * 8 + x) as u8;
                result[square][direction].len += 1;
                x += dx;
                y += dy;
            }
            direction += 1;
        }
        square += 1;
    }
    result
}

pub(crate) static KNIGHT_TARGETS: [Targets; 64] = targets(&KNIGHT_STEPS);
pub(crate) static KING_TARGETS: [Targets; 64] = targets(&DIRECTIONS);
// Index by attacking color (black = 0, white = 1).
pub(crate) static PAWN_ATTACKERS: [[Targets; 64]; 2] =
    [targets(&[(-1, 1), (1, 1)]), targets(&[(-1, -1), (1, -1)])];
pub(crate) static RAYS: [[Targets; 8]; 64] = rays();
