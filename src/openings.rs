//! Bundled opening names matched against the played SAN sequence.

mod data;
use data::OPENING_LINES;

fn normalized(move_san: &str) -> impl Iterator<Item = char> + '_ {
    move_san
        .trim_end_matches(&['+', '#', '!', '?'][..])
        .chars()
        .map(|c| if c == '0' { 'O' } else { c })
}

pub(crate) fn opening_name(notation: &[String]) -> &'static str {
    let mut name = if notation.is_empty() {
        "Starting position"
    } else {
        "Unclassified opening"
    };
    let mut depth = 0;
    for &(label, line) in OPENING_LINES {
        if line.len() <= depth || line.len() > notation.len() {
            continue;
        }
        if line
            .iter()
            .zip(notation)
            .all(|(expected, played)| normalized(expected).eq(normalized(played)))
        {
            // Labels begin with an ECO code or range followed by the name.
            name = label.split_once(' ').unwrap().1;
            depth = line.len();
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(moves: &[&str]) -> &'static str {
        opening_name(&moves.iter().map(|m| (*m).to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn recognizes_every_bundled_line() {
        assert_eq!(OPENING_LINES.len(), 3865);
        for &(label, line) in OPENING_LINES {
            assert_eq!(name(line), label.split_once(' ').unwrap().1, "{label}");
        }
    }

    #[test]
    fn recognizes_deep_variations_from_played_moves() {
        let expected = "Ruy Lopez: Marshall Attack, Main Line, Spassky Variation";
        let &(_, line) = OPENING_LINES
            .iter()
            .find(|(label, _)| label.split_once(' ').unwrap().1 == expected)
            .unwrap();
        let mut game = crate::game::Game::default();
        for san in line {
            let position = game.position();
            let legal = position.legal_moves();
            let m = legal
                .iter()
                .copied()
                .find(|&m| normalized(&position.san(m, &legal)).eq(normalized(san)))
                .unwrap_or_else(|| panic!("Missing legal move {san}"));
            game.play(m).unwrap();
        }
        assert_eq!(game.moves.len(), 36);
        assert_eq!(game.opening_name(), expected);
        assert!(game.undo());
        assert_ne!(game.opening_name(), expected);
    }

    #[test]
    fn matches_deepest_complete_line_and_retains_it_after_deviation() {
        assert_eq!(name(&[]), "Starting position");
        assert_eq!(name(&["invalid"]), "Unclassified opening");
        assert_eq!(name(&["e4", "c5"]), "Sicilian Defense");
        assert_eq!(name(&["b4"]), "Polish Opening");
        assert_eq!(name(&["b4", "Nh6"]), "Polish Opening: Karniewski Variation");
        assert_eq!(name(&["b4", "Nh6", "a3"]), name(&["b4", "Nh6"]));
        assert_eq!(
            name(&["e4", "e5", "Nf3", "Nc6", "Bb5", "a6", "Ba4", "Nf6", "O-O"]),
            "Ruy Lopez: Morphy Defense"
        );
        // Alternate move orders only match when explicitly listed.
        assert_ne!(name(&["c5", "e4"]), "Sicilian Defense");
    }

    #[test]
    fn normalizes_checks_annotations_and_castling_in_both_inputs() {
        assert_eq!(name(&["e4+#!?"]), name(&["e4"]));
        assert_eq!(
            name(&["e4", "e5", "Nf3", "d6", "d4", "Nd7", "Bc4", "c6", "0-0!?"]),
            "Philidor Defense: Hanham Variation, Krause Variation"
        );
        // This bundled line includes a check suffix that SAN may omit.
        assert_eq!(
            name(&["e4", "e5", "Nf3", "Nc6", "Bc4", "Bc5", "Bxf7"]),
            "Italian Game: Jerome Gambit"
        );
    }
}
