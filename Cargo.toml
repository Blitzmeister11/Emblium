// Turnier- und SPRT-Modul.
// Enthält: headless Engine-vs-Engine-Partien, Pairing-Generierung (Round-Robin, Gauntlet),
// und die SPRT/LLR/Elo-Statistik zum Vergleich zweier Engine-Versionen.

use crate::uci_engine::{EngineEvent, UciEngine};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    WhiteWin,
    BlackWin,
    Draw,
}

#[derive(Debug, Clone)]
pub struct GameResult {
    pub outcome: Outcome,
    pub moves: Vec<String>,
    pub termination: String,
}

const MAX_PLIES: usize = 400; // Sicherheitsgrenze gegen Endlospartien

/// Spielt eine vollständige Partie zwischen zwei UCI-Engines (headless, ohne GUI).
/// `white_path` zieht Weiß, `black_path` zieht Schwarz.
pub fn play_game(
    white_path: &str,
    black_path: &str,
    movetime_ms: u32,
) -> Result<GameResult, String> {
    let mut white = UciEngine::start(white_path).map_err(|e| e.to_string())?;
    let mut black = UciEngine::start(black_path).map_err(|e| e.to_string())?;

    wait_for_ready(&white)?;
    wait_for_ready(&black)?;
    white.new_game().map_err(|e| e.to_string())?;
    black.new_game().map_err(|e| e.to_string())?;
    wait_for_ready(&white)?;
    wait_for_ready(&black)?;

    let mut board = shakmaty::Chess::default();
    let mut uci_moves: Vec<String> = Vec::new();

    let result = loop {
        use shakmaty::Position;

        if let Some(outcome) = terminal_outcome(&board) {
            break Ok(GameResult {
                outcome,
                moves: uci_moves.clone(),
                termination: "Normal".to_string(),
            });
        }
        if uci_moves.len() >= MAX_PLIES {
            break Ok(GameResult {
                outcome: Outcome::Draw,
                moves: uci_moves.clone(),
                termination: "Zuglimit erreicht".to_string(),
            });
        }

        let white_to_move = board.turn() == shakmaty::Color::White;
        let engine = if white_to_move { &mut white } else { &mut black };

        engine.set_position(&uci_moves).map_err(|e| e.to_string())?;
        engine.go_movetime(movetime_ms).map_err(|e| e.to_string())?;

        let best = match wait_for_bestmove(engine, movetime_ms as u64 + 5000)? {
            Some(m) => m,
            None => {
                break Ok(GameResult {
                    outcome: if white_to_move { Outcome::BlackWin } else { Outcome::WhiteWin },
                    moves: uci_moves.clone(),
                    termination: "Engine lieferte keinen Zug (Absturz/Timeout)".to_string(),
                })
            }
        };

        let uci: shakmaty::uci::UciMove = match best.parse() {
            Ok(u) => u,
            Err(_) => {
                break Ok(GameResult {
                    outcome: if white_to_move { Outcome::BlackWin } else { Outcome::WhiteWin },
                    moves: uci_moves.clone(),
                    termination: format!("Ungültiger Zugstring von Engine: {best}"),
                })
            }
        };
        let mv = match uci.to_move(&board) {
            Ok(m) => m,
            Err(_) => {
                break Ok(GameResult {
                    outcome: if white_to_move { Outcome::BlackWin } else { Outcome::WhiteWin },
                    moves: uci_moves.clone(),
                    termination: format!("Illegaler Zug von Engine: {best}"),
                })
            }
        };
        board = board.play(&mv).map_err(|e| e.to_string())?;
        uci_moves.push(best);
    };

    white.quit();
    black.quit();
    result
}

fn terminal_outcome(board: &shakmaty::Chess) -> Option<Outcome> {
    use shakmaty::Position;
    if board.is_checkmate() {
        return Some(if board.turn() == shakmaty::Color::White {
            Outcome::BlackWin
        } else {
            Outcome::WhiteWin
        });
    }
    if board.is_stalemate() || board.is_insufficient_material() || board.halfmoves() >= 100 {
        return Some(Outcome::Draw);
    }
    None
}

fn wait_for_ready(engine: &UciEngine) -> Result<(), String> {
    let deadline = Duration::from_secs(10);
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if let Ok(EngineEvent::Ready) = engine.events.recv_timeout(Duration::from_millis(200)) {
            return Ok(());
        }
    }
    Err("Engine wurde nicht rechtzeitig bereit".to_string())
}

fn wait_for_bestmove(engine: &UciEngine, timeout_ms: u64) -> Result<Option<String>, String> {
    let deadline = Duration::from_millis(timeout_ms);
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        match engine.events.recv_timeout(Duration::from_millis(200)) {
            Ok(EngineEvent::BestMove { best, .. }) => {
                return Ok(if best == "(none)" { None } else { Some(best) })
            }
            Ok(EngineEvent::Crashed(_)) => return Ok(None),
            _ => continue,
        }
    }
    Ok(None)
}

// ---------- Pairings ----------

/// Jede Engine spielt gegen jede andere (einmal als Weiß, einmal als Schwarz).
/// Rückgabe: Liste von (weißer_index, schwarzer_index).
pub fn round_robin_pairs(n_engines: usize) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for i in 0..n_engines {
        for j in 0..n_engines {
            if i != j {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

/// Eine Test-Engine (Index `test_idx`) spielt gegen jeden Gegner in `opponents`,
/// jeweils einmal als Weiß und einmal als Schwarz.
pub fn gauntlet_pairs(test_idx: usize, opponents: &[usize]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for &opp in opponents {
        pairs.push((test_idx, opp));
        pairs.push((opp, test_idx));
    }
    pairs
}

// ---------- SPRT / LLR / Elo ----------

/// Log-Likelihood-Ratio für SPRT, nach der von cutechess-cli/fastchess verwendeten
/// Trinomial-Formel (Wins/Draws/Losses gegen zwei Elo-Hypothesen elo0 < elo1).
pub struct SprtState {
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub elo0: f64,
    pub elo1: f64,
    pub alpha: f64,
    pub beta: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SprtDecision {
    AcceptH1, // Verbesserung bestätigt
    AcceptH0, // Keine Verbesserung
    Continue,
}

impl SprtState {
    pub fn new(elo0: f64, elo1: f64, alpha: f64, beta: f64) -> Self {
        SprtState { wins: 0, draws: 0, losses: 0, elo0, elo1, alpha, beta }
    }

    pub fn record(&mut self, outcome_for_test_engine: Outcome) {
        match outcome_for_test_engine {
            Outcome::WhiteWin => self.wins += 1,
            Outcome::BlackWin => self.losses += 1,
            Outcome::Draw => self.draws += 1,
        }
    }

    fn total(&self) -> u32 {
        self.wins + self.draws + self.losses
    }

    /// LLR über die Pentanomial-Näherung durch Normalapproximation der Score-Verteilung.
    /// Score-Erwartungswert und Varianz aus W/D/L, verglichen zwischen den beiden
    /// Elo-Hypothesen elo0/elo1 (umgerechnet in erwartete Score-Werte).
    pub fn llr(&self) -> f64 {
        let n = self.total() as f64;
        if n == 0.0 {
            return 0.0;
        }
        let score = (self.wins as f64 + 0.5 * self.draws as f64) / n;
        let draw_rate = self.draws as f64 / n;

        let var = score * (1.0 - score) - 0.25 * draw_rate * draw_rate;
        let var = var.max(1e-8);

        let s0 = elo_to_score(self.elo0);
        let s1 = elo_to_score(self.elo1);

        // Normalapproximation der LLR (Standardverfahren, wie u.a. bei cutechess/SPRT-Rechnern
        // mit Pentanomial/Trinomial-Modell für die Score-Verteilung genutzt).
        n * (s1 - s0) * (2.0 * score - s0 - s1) / (2.0 * var)
    }

    pub fn bounds(&self) -> (f64, f64) {
        let lower = (self.beta / (1.0 - self.alpha)).ln();
        let upper = ((1.0 - self.beta) / self.alpha).ln();
        (lower, upper)
    }

    pub fn decide(&self) -> SprtDecision {
        let llr = self.llr();
        let (lower, upper) = self.bounds();
        if llr >= upper {
            SprtDecision::AcceptH1
        } else if llr <= lower {
            SprtDecision::AcceptH0
        } else {
            SprtDecision::Continue
        }
    }

    pub fn estimated_elo(&self) -> Option<(f64, f64)> {
        let n = self.total() as f64;
        if n < 1.0 {
            return None;
        }
        let score = (self.wins as f64 + 0.5 * self.draws as f64) / n;
        if score <= 0.0 || score >= 1.0 {
            return None;
        }
        let elo = -400.0 * (1.0 / score - 1.0).log10();
        // grobe Standardabweichung der Score-Schätzung -> Elo-Fehlerbalken
        let draw_rate = self.draws as f64 / n;
        let var = (score * (1.0 - score) - 0.25 * draw_rate * draw_rate).max(1e-8);
        let se_score = (var / n).sqrt();
        let elo_hi = -400.0 * (1.0 / (score + se_score).min(0.999999) - 1.0).log10();
        let elo_lo = -400.0 * (1.0 / (score - se_score).max(0.000001) - 1.0).log10();
        Some((elo, (elo_hi - elo_lo).abs() / 2.0))
    }
}

fn elo_to_score(elo: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-elo / 400.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_pair_count() {
        // n Engines -> n*(n-1) Paarungen (jede Farbe einmal)
        assert_eq!(round_robin_pairs(3).len(), 6);
        assert_eq!(round_robin_pairs(4).len(), 12);
    }

    #[test]
    fn gauntlet_pair_count() {
        let pairs = gauntlet_pairs(0, &[1, 2, 3]);
        assert_eq!(pairs.len(), 6);
        assert!(pairs.contains(&(0, 1)));
        assert!(pairs.contains(&(1, 0)));
    }

    #[test]
    fn elo_to_score_symmetry() {
        // Bei elo=0 muss der erwartete Score exakt 0.5 sein
        assert!((elo_to_score(0.0) - 0.5).abs() < 1e-9);
        // Score muss mit steigender Elo-Differenz monoton wachsen
        assert!(elo_to_score(50.0) > elo_to_score(0.0));
        assert!(elo_to_score(-50.0) < elo_to_score(0.0));
    }

    #[test]
    fn sprt_llr_zero_at_neutral_score() {
        // Score exakt in der Mitte zwischen s0 und s1 -> LLR ~ 0
        let mut state = SprtState::new(0.0, 10.0, 0.05, 0.05);
        let s0 = elo_to_score(0.0);
        let s1 = elo_to_score(10.0);
        let mid = (s0 + s1) / 2.0;
        let n = 1000u32;
        state.wins = (mid * n as f64) as u32;
        state.losses = n - state.wins;
        state.draws = 0;
        assert!(state.llr().abs() < 0.5, "LLR war {}", state.llr());
    }

    #[test]
    fn sprt_llr_positive_when_favoring_h1() {
        let mut state = SprtState::new(0.0, 10.0, 0.05, 0.05);
        state.wins = 600;
        state.draws = 200;
        state.losses = 200;
        assert!(state.llr() > 0.0, "LLR war {} (sollte > 0 sein)", state.llr());
    }

    #[test]
    fn sprt_bounds_reasonable() {
        let state = SprtState::new(0.0, 10.0, 0.05, 0.05);
        let (lower, upper) = state.bounds();
        assert!(lower < 0.0);
        assert!(upper > 0.0);
    }

    #[test]
    fn estimated_elo_positive_when_winning_more() {
        let mut state = SprtState::new(0.0, 10.0, 0.05, 0.05);
        state.wins = 60;
        state.draws = 20;
        state.losses = 20;
        let (elo, _err) = state.estimated_elo().unwrap();
        assert!(elo > 0.0);
    }

    #[test]
    fn play_game_stockfish_vs_stockfish_terminates() {
        if std::process::Command::new("/usr/games/stockfish").spawn().is_err() {
            eprintln!("Stockfish nicht gefunden, Test übersprungen");
            return;
        }
        let result = play_game("/usr/games/stockfish", "/usr/games/stockfish", 50).unwrap();
        assert!(!result.moves.is_empty(), "Partie hatte keine Züge");
        println!(
            "Ergebnis: {:?}, {} Halbzüge, Grund: {}",
            result.outcome,
            result.moves.len(),
            result.termination
        );
    }
}
