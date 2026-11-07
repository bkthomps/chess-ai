Chess regression positions for legal move counts, special moves, material draws, checkmate, and stalemate.
Four-field positions have been completed with `0 1` for the clocks.

The en passant cases count every legal pawn push and capture, along with all king moves.
King and bishop against king and bishop is a material draw when both bishops occupy the same square color.

Recursive tests count special moves at the final ply, and verify incremental Zobrist hashes at every edge.
