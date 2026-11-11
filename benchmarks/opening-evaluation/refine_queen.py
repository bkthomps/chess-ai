"""Contextual queen trials; temporary builds, preserving the application engine."""
import sys

import followup_constants as followup

REFINEMENTS = {}
needle, blanket = followup.VARIANTS['early-queen24'][0]
for weight in [16, 24, 32]:
    old = 'early_queen += sign * queen.count_ones() as i32 * undeveloped * 24;'
    assert old in blanket
    # General geometry: exposure rises from an edge file toward central files.
    # No named opening or special handling of a5/e6 is used.
    replacement = '''let mut queens = queen;
        while queens != 0 {
            let square = queens.trailing_zeros() as usize;
            queens &= queens - 1;
            let file = square % 8;
            let exposure = file.min(7 - file) as i32;
            early_queen += sign * undeveloped * WEIGHT * exposure / 3;
        }'''.replace('WEIGHT', str(weight))
    REFINEMENTS[f'queen-file-{weight}'] = [(needle, blanket.replace(old, replacement))]

for power in [2, 3]:
    for weight in [24, 32]:
        replacement = '''let mut queens = queen;
        while queens != 0 {
            let square = queens.trailing_zeros() as usize;
            queens &= queens - 1;
            let file = square % 8;
            let exposure = file.min(7 - file) as i32;
            early_queen += sign * undeveloped * WEIGHT * exposure.pow(POWER) / DIVISOR;
        }'''.replace('WEIGHT', str(weight)).replace('POWER', str(power)).replace('DIVISOR', str(3 ** power))
        REFINEMENTS[f'queen-file-power{power}-{weight}'] = [(needle, blanket.replace(old, replacement))]

CHASE = '''
fn experimental_queen_chase(
    p: &Position, occupied: [u64; 2], pawns: [u64; 2], kings: [usize; 2],
    knights: [u64; 2], diagonal: [u64; 2], straight: [u64; 2],
    pawn_attacks: [u64; 2], pressure: &Pressure,
) -> i32 {
    let mobility = mobility::Mobility::from_masks(
        occupied, pawns, kings, diagonal, straight, pawn_attacks);
    let mut total = 0;
    for color in 0..2 {
        let enemy = 1 - color;
        let mut queens = diagonal[color] & straight[color];
        while queens != 0 {
            let square = queens.trailing_zeros() as usize;
            let queen_bit = 1u64 << square;
            queens &= queens - 1;
            // Current attacks already receive the ordinary threat score.
            if (pressure.minor[enemy] | pawn_attacks[enemy]) & queen_bit != 0 {
                continue;
            }
            let safe = !occupied[enemy] & !queen_bit & !pawn_attacks[color]
                & (!pressure.all[color] | pressure.all[enemy]);
            let knight_targets = crate::tables::KNIGHT_TARGETS[square].iter()
                .fold(0, |bits, to| bits | (1u64 << to));
            let mut attackers = knights[enemy];
            if BISHOPS {
                attackers |= diagonal[enemy] & !straight[enemy];
            }
            let mut can_chase = false;
            while attackers != 0 {
                let from = attackers.trailing_zeros() as usize;
                attackers &= attackers - 1;
                let pc = p.squares[from];
                let destinations = mobility.attacks(from, pc) & safe;
                let targets = if pc.abs() == 2 {
                    knight_targets
                } else {
                    // Reverse bishop rays stop at the current first blocker.
                    mobility.attacks(square, if color == 1 { 3 } else { -3 })
                };
                if destinations & targets != 0 {
                    can_chase = true;
                    break;
                }
            }
            if can_chase {
                total += if color == 1 { -WEIGHT } else { WEIGHT };
            }
        }
    }
    total
}
'''
for family, bishops, weights in [('knight-chase', False, [16, 24, 32]),
                                 ('minor-chase', True, [16, 24])]:
    for weight in weights:
        helper = CHASE.replace('BISHOPS', str(bishops).lower()).replace('WEIGHT', str(weight))
        REFINEMENTS[f'{family}-{weight}'] = [
            ('fn threats(', helper + '\nfn threats('),
            ('let (threat_mg, threat_eg) = threats(p, occupied, kings, pawn_attacks, &pressure);',
             'let (base_threat, threat_eg) = threats(p, occupied, kings, pawn_attacks, &pressure);\n'
             '    let threat_mg = base_threat + experimental_queen_chase(p, occupied, pawns, kings, knights, diagonal, straight, pawn_attacks, &pressure);')]

followup.VARIANTS.update(REFINEMENTS)

if __name__ == '__main__':
    if '--variants' not in sys.argv:
        sys.argv[1:1] = ['--variants', 'current', 'early-queen24', *REFINEMENTS]
    followup.main()
