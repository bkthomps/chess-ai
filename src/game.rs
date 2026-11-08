//! Game history, notation, and terminal-state adjudication.
use crate::chess::{Move, Position};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Checkmate { winner_white: bool },
    Stalemate,
    Repetition,
    FiftyMove,
    InsufficientMaterial,
}
impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Checkmate { winner_white: true } => "White wins by checkmate",
            Self::Checkmate {
                winner_white: false,
            } => "Black wins by checkmate",
            Self::Stalemate => "Draw by stalemate",
            Self::Repetition => "Draw by threefold repetition",
            Self::FiftyMove => "Draw by the fifty-move rule",
            Self::InsufficientMaterial => "Draw by insufficient material",
        }
    }
}

pub fn outcome(position: &Position, keys: &[u64], legal: &[Move]) -> Option<Outcome> {
    if legal.is_empty() {
        return Some(if position.in_check(position.white) {
            Outcome::Checkmate {
                winner_white: !position.white,
            }
        } else {
            Outcome::Stalemate
        });
    }
    if position.halfmove >= 100 {
        return Some(Outcome::FiftyMove);
    }
    if position.insufficient_material() {
        return Some(Outcome::InsufficientMaterial);
    }
    let key = position.repetition_key();
    if keys.iter().filter(|&&k| k == key).count() >= 3 {
        return Some(Outcome::Repetition);
    }
    None
}

pub fn timeout_result(position: &Position, flagged_white: bool) -> String {
    if position.insufficient_mating_material(!flagged_white) {
        "Draw on time: opponent has insufficient mating material".into()
    } else {
        format!(
            "{} wins on time",
            if flagged_white { "Black" } else { "White" }
        )
    }
}

#[derive(Clone)]
pub struct Game {
    pub positions: Vec<Position>,
    pub keys: Vec<u64>,
    pub moves: Vec<Move>,
    pub notation: Vec<String>,
}
impl Default for Game {
    fn default() -> Self {
        Self::new(Position::default())
    }
}
impl Game {
    pub fn new(position: Position) -> Self {
        Self {
            keys: vec![position.repetition_key()],
            positions: vec![position],
            moves: vec![],
            notation: vec![],
        }
    }
    pub fn position(&self) -> &Position {
        self.positions.last().unwrap()
    }
    /// Deepest listed opening in the played sequence, retained after deviations.
    pub fn opening_name(&self) -> &'static str {
        crate::openings::opening_name(&self.notation)
    }
    pub fn result(&self) -> Option<Outcome> {
        outcome(self.position(), &self.keys, &self.position().legal_moves())
    }
    pub fn play(&mut self, m: Move) -> Result<(), String> {
        let legal = self.position().legal_moves();
        if outcome(self.position(), &self.keys, &legal).is_some() {
            return Err("The game has ended.".into());
        }
        if !legal.contains(&m) {
            return Err("That move is not legal.".into());
        }
        let san = self.position().san(m, &legal);
        let next = self.position().play(m);
        self.keys.push(next.repetition_key());
        self.positions.push(next);
        self.moves.push(m);
        self.notation.push(san);
        Ok(())
    }
    pub fn undo(&mut self) -> bool {
        if self.moves.pop().is_none() {
            return false;
        }
        self.positions.pop();
        self.keys.pop();
        self.notation.pop();
        true
    }
}
