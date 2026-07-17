// Board-Zustand: Spiellogik über shakmaty, reines Zeichnen über fltk::draw.

use fltk::draw;
use fltk::enums::{Align, Color as FltkColor, Font};
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Move, Piece, Position, Role, Square};

pub struct BoardState {
    pub position: Chess,
    pub history: Vec<Move>,
    pub selected: Option<Square>,
    pub legal_targets: Vec<Square>,
    pub flipped: bool,
}

impl BoardState {
    pub fn new() -> Self {
        BoardState {
            position: Chess::default(),
            history: Vec::new(),
            selected: None,
            legal_targets: Vec::new(),
            flipped: false,
        }
    }

    pub fn reset(&mut self) {
        self.position = Chess::default();
        self.history.clear();
        self.selected = None;
        self.legal_targets.clear();
    }

    pub fn fen(&self) -> String {
        Fen::from_position(self.position.clone(), EnPassantMode::Legal).to_string()
    }

    pub fn uci_history(&self) -> Vec<String> {
        self.history
            .iter()
            .map(|m| m.to_uci(CastlingMode::Standard).to_string())
            .collect()
    }

    /// Versucht, den vom Menschen angeklickten Zug (from -> to) zu spielen.
    /// Bei Bauernumwandlung wird automatisch auf Dame promoviert.
    /// Gibt den gespielten Move zurück, falls legal.
    pub fn try_human_move(&mut self, from: Square, to: Square) -> Option<Move> {
        let legals = self.position.legal_moves();
        let candidate = legals.iter().find(|m| {
            m.from() == Some(from)
                && m.to() == to
                && (m.promotion().is_none() || m.promotion() == Some(Role::Queen))
        })?;
        let played = candidate.clone();
        self.position = self.position.clone().play(&played).ok()?;
        self.history.push(played.clone());
        Some(played)
    }

    pub fn push_uci(&mut self, uci_str: &str) -> Result<(), String> {
        let uci: shakmaty::uci::UciMove = uci_str
            .parse()
            .map_err(|_| format!("Ungültiger UCI-Zug: {uci_str}"))?;
        let m = uci
            .to_move(&self.position)
            .map_err(|_| format!("Illegaler Zug: {uci_str}"))?;
        self.position = self
            .position
            .clone()
            .play(&m)
            .map_err(|_| format!("Zug konnte nicht gespielt werden: {uci_str}"))?;
        self.history.push(m);
        Ok(())
    }

    pub fn game_over_message(&self) -> Option<String> {
        if self.position.is_checkmate() {
            let winner = if self.position.turn() == Color::White { "Schwarz" } else { "Weiß" };
            Some(format!("Matt – {winner} gewinnt"))
        } else if self.position.is_stalemate() {
            Some("Patt – Remis".to_string())
        } else if self.position.is_insufficient_material() {
            Some("Remis – ungenügendes Material".to_string())
        } else if self.position.halfmoves() >= 100 {
            Some("Remis – 50-Züge-Regel".to_string())
        } else {
            None
        }
    }

    fn to_screen(&self, file: u32, rank: u32) -> (u32, u32) {
        if self.flipped {
            (7 - file, rank)
        } else {
            (file, 7 - rank)
        }
    }

    fn from_screen(&self, col: u32, row: u32) -> Square {
        let (file, rank) = if self.flipped {
            (7 - col, row)
        } else {
            (col, 7 - row)
        };
        Square::from_coords(
            shakmaty::File::new(file),
            shakmaty::Rank::new(rank),
        )
    }

    pub fn square_at_pixel(&self, x: i32, y: i32, origin_x: i32, origin_y: i32, square_size: i32) -> Option<Square> {
        let rel_x = x - origin_x;
        let rel_y = y - origin_y;
        if rel_x < 0 || rel_y < 0 {
            return None;
        }
        let col = rel_x / square_size;
        let row = rel_y / square_size;
        if col > 7 || row > 7 {
            return None;
        }
        Some(self.from_screen(col as u32, row as u32))
    }

    pub fn handle_click(&mut self, clicked: Square) -> Option<Move> {
        match self.selected {
            None => {
                if let Some(piece) = self.position.board().piece_at(clicked) {
                    if piece.color == self.position.turn() {
                        self.selected = Some(clicked);
                        self.legal_targets = self
                            .position
                            .legal_moves()
                            .iter()
                            .filter(|m| m.from() == Some(clicked))
                            .map(|m| m.to())
                            .collect();
                    }
                }
                None
            }
            Some(from) => {
                let result = self.try_human_move(from, clicked);
                self.selected = None;
                self.legal_targets.clear();
                result
            }
        }
    }

    pub fn draw(&self, x: i32, y: i32, w: i32, h: i32) {
        let size = w.min(h);
        let square_size = size / 8;

        for rank in 0..8u32 {
            for file in 0..8u32 {
                let (sx, sy) = self.to_screen(file, rank);
                let px = x + sx as i32 * square_size;
                let py = y + sy as i32 * square_size;
                let square = Square::from_coords(shakmaty::File::new(file), shakmaty::Rank::new(rank));

                let is_selected = self.selected == Some(square);
                let base_color = if (file + rank) % 2 == 0 {
                    FltkColor::from_rgb(0x76, 0x96, 0x56)
                } else {
                    FltkColor::from_rgb(0xEE, 0xEE, 0xD2)
                };
                let color = if is_selected {
                    FltkColor::from_rgb(0xF6, 0xF6, 0x69)
                } else {
                    base_color
                };
                draw::draw_rect_fill(px, py, square_size, square_size, color);

                if let Some(piece) = self.position.board().piece_at(square) {
                    draw_piece(piece, px, py, square_size);
                }

                if self.legal_targets.contains(&square) {
                    draw::set_draw_color(FltkColor::from_rgba_tuple((20, 20, 20, 130)));
                    let r = square_size / 6;
                    draw::draw_pie(
                        px + square_size / 2 - r,
                        py + square_size / 2 - r,
                        r * 2,
                        r * 2,
                        0.0,
                        360.0,
                    );
                }
            }
        }
    }
}

fn draw_piece(piece: Piece, px: i32, py: i32, square_size: i32) {
    let symbol = piece_symbol(piece);
    draw::set_font(Font::Helvetica, (square_size as f64 * 0.65) as i32);
    draw::set_draw_color(if piece.color == Color::White {
        FltkColor::from_rgb(250, 250, 250)
    } else {
        FltkColor::from_rgb(20, 20, 20)
    });
    draw::draw_text2(
        symbol,
        px,
        py,
        square_size,
        square_size,
        Align::Center,
    );
}

fn piece_symbol(piece: Piece) -> &'static str {
    match (piece.color, piece.role) {
        (Color::White, Role::Pawn) => "\u{2659}",
        (Color::White, Role::Knight) => "\u{2658}",
        (Color::White, Role::Bishop) => "\u{2657}",
        (Color::White, Role::Rook) => "\u{2656}",
        (Color::White, Role::Queen) => "\u{2655}",
        (Color::White, Role::King) => "\u{2654}",
        (Color::Black, Role::Pawn) => "\u{265F}",
        (Color::Black, Role::Knight) => "\u{265E}",
        (Color::Black, Role::Bishop) => "\u{265D}",
        (Color::Black, Role::Rook) => "\u{265C}",
        (Color::Black, Role::Queen) => "\u{265B}",
        (Color::Black, Role::King) => "\u{265A}",
    }
}
