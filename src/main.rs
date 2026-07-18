#![cfg_attr(windows, windows_subsystem = "windows")]

use emblium::board;
use emblium::uci_engine;

use std::cell::RefCell;
use std::rc::Rc;

use fltk::app;
use fltk::browser::HoldBrowser;
use fltk::button::{Button, CheckButton};
use fltk::dialog;
use fltk::enums::{Align, Event, FrameType};
use fltk::frame::Frame;
use fltk::group::Pack;
use fltk::input::IntInput;
use fltk::menu::Choice;
use fltk::misc::Progress;
use fltk::prelude::*;
use fltk::text::{TextBuffer, TextDisplay};
use fltk::window::Window;
use std::path::PathBuf;

use board::BoardState;
use shakmaty::Position;
use uci_engine::{EngineEvent, UciEngine};

const DEFAULT_MOVE_TIME_MS: u32 = 1000;

fn config_file_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.set_file_name("emble_gui_last_engine.txt");
    p
}

fn load_last_engine_path() -> Option<String> {
    std::fs::read_to_string(config_file_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn save_last_engine_path(path: &str) {
    let _ = std::fs::write(config_file_path(), path);
}

struct AppState {
    board: BoardState,
    engine: Option<UciEngine>,
    human_is_white: bool,
    waiting_for_engine: bool,
    game_over: bool,
    movetime_ms: u32,
    infinite: bool,
}

impl AppState {
    fn new() -> Self {
        AppState {
            board: BoardState::new(),
            engine: None,
            human_is_white: true,
            waiting_for_engine: false,
            game_over: false,
            movetime_ms: DEFAULT_MOVE_TIME_MS,
            infinite: false,
        }
    }
}

fn main() {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    app::set_visible_focus(false);
    app::background(0x1E, 0x1E, 0x28);
    app::background2(0x2A, 0x2A, 0x38);
    app::foreground(0xE8, 0xE8, 0xEC);
    app::set_font_size(14);
    let mut win = Window::new(100, 100, 1050, 700, "Emblium");
    win.set_color(fltk::enums::Color::from_rgb(0x1E, 0x1E, 0x28));

    let state = Rc::new(RefCell::new(AppState::new()));

    // ---------- Board (links) ----------
    let mut board_frame = Frame::new(20, 20, 600, 600, "");
    board_frame.set_frame(FrameType::FlatBox);

    // ---------- Seitenpanel (rechts) ----------
    let mut side = Pack::new(640, 20, 390, 660, "");
    side.set_spacing(8);

    let mut load_engine_btn = Button::new(0, 0, 390, 30, "Engine laden...");
    style_accent_button(&mut load_engine_btn);
    let mut engine_status = Frame::new(0, 0, 390, 24, "Keine Engine geladen");
    engine_status.set_align(Align::Left | Align::Inside);

    let mut color_choice = Choice::new(0, 0, 390, 28, "");
    color_choice.add_choice("Mensch spielt Weiß");
    color_choice.add_choice("Mensch spielt Schwarz");
    color_choice.set_value(0);
    color_choice.set_frame(FrameType::FlatBox);
    color_choice.set_color(fltk::enums::Color::from_rgb(0x2A, 0x2A, 0x38));
    color_choice.set_text_color(fltk::enums::Color::from_rgb(0xE8, 0xE8, 0xEC));

    let mut new_game_btn = Button::new(0, 0, 390, 30, "Neue Partie");
    style_accent_button(&mut new_game_btn);
    let mut flip_btn = Button::new(0, 0, 390, 30, "Brett drehen");
    style_flat_button(&mut flip_btn);
    let mut tournament_btn = Button::new(0, 0, 390, 30, "Turnier & SPRT öffnen...");
    style_flat_button(&mut tournament_btn);

    let time_row_label = Frame::new(0, 0, 390, 18, "Bedenkzeit der Engine pro Zug:");
    let mut time_row = Pack::new(0, 0, 390, 28, "");
    time_row.set_type(fltk::group::PackType::Horizontal);
    time_row.set_spacing(8);
    let mut movetime_input = IntInput::new(0, 0, 100, 28, "");
    movetime_input.set_value(&DEFAULT_MOVE_TIME_MS.to_string());
    movetime_input.set_frame(FrameType::FlatBox);
    movetime_input.set_color(fltk::enums::Color::from_rgb(0x2A, 0x2A, 0x38));
    movetime_input.set_text_color(fltk::enums::Color::from_rgb(0xE8, 0xE8, 0xEC));
    let ms_label = Frame::new(0, 0, 40, 28, "ms");
    let mut infinite_check = CheckButton::new(0, 0, 120, 28, "Unendlich");
    infinite_check.set_selection_color(fltk::enums::Color::from_rgb(0x5B, 0x8D, 0xEF));
    time_row.end();
    let mut stop_btn = Button::new(0, 0, 390, 28, "Engine anhalten (Stopp)");
    style_warning_button(&mut stop_btn);

    let eval_label = Frame::new(0, 0, 390, 20, "Eval (Bauerneinheiten, Sicht Weiß):");
    let mut eval_bar = Progress::new(0, 0, 390, 24, "");
    eval_bar.set_minimum(-1000.0);
    eval_bar.set_maximum(1000.0);
    eval_bar.set_value(0.0);
    eval_bar.set_label("0.00");
    eval_bar.set_frame(FrameType::FlatBox);
    eval_bar.set_color(fltk::enums::Color::from_rgb(0x2A, 0x2A, 0x38));
    eval_bar.set_selection_color(fltk::enums::Color::from_rgb(0x5B, 0x8D, 0xEF));

    let move_list_label = Frame::new(0, 0, 390, 20, "Zugliste:");
    let mut move_list = HoldBrowser::new(0, 0, 390, 160, "");
    move_list.set_frame(FrameType::FlatBox);
    move_list.set_color(fltk::enums::Color::from_rgb(0x2A, 0x2A, 0x38));

    let output_label = Frame::new(0, 0, 390, 20, "Engine-Ausgabe:");
    let mut engine_output = TextDisplay::new(0, 0, 390, 260, "");
    let output_buffer = TextBuffer::default();
    engine_output.set_buffer(output_buffer.clone());
    engine_output.set_frame(FrameType::FlatBox);
    engine_output.set_color(fltk::enums::Color::from_rgb(0x2A, 0x2A, 0x38));
    engine_output.set_text_color(fltk::enums::Color::from_rgb(0xE8, 0xE8, 0xEC));

    side.end();
    win.end();
    win.show();

    board_frame.set_color(fltk::enums::Color::Black);

    // ---------- Zeichnen ----------
    {
        let state = state.clone();
        board_frame.draw(move |f| {
            let st = state.borrow();
            st.board.draw(f.x(), f.y(), f.w(), f.h());
        });
    }

    // ---------- Klick-Handling ----------
    {
        let state = state.clone();
        let mut move_list_h = move_list.clone();
        let mut board_frame_h = board_frame.clone();
        board_frame.handle(move |f, ev| {
            if ev == Event::Push {
                let (mx, my) = app::event_coords();
                let size = f.w().min(f.h());
                let square_size = size / 8;
                let mut st = state.borrow_mut();
                if let Some(sq) = st.board.square_at_pixel(mx, my, f.x(), f.y(), square_size) {
                    if !st.waiting_for_engine && !st.game_over {
                        if let Some(_mv) = st.board.handle_click(sq) {
                            refresh_move_list(&st.board, &mut move_list_h);
                            check_game_over(&mut st);
                            if !st.game_over {
                                request_engine_move(&mut st);
                            }
                        }
                    }
                }
                drop(st);
                f.redraw();
                board_frame_h.redraw();
                true
            } else {
                false
            }
        });
    }

    // ---------- Engine laden ----------
    {
        let state = state.clone();
        let mut engine_status_h = engine_status.clone();
        load_engine_btn.set_callback(move |_| {
            let mut chooser = dialog::NativeFileChooser::new(dialog::FileDialogType::BrowseFile);
            chooser.set_title("UCI-Engine auswählen");
            chooser.show();
            let path = chooser.filename();
            if path.as_os_str().is_empty() {
                return;
            }
            let path = path.to_string_lossy().to_string();
            try_load_engine(&path, &state, &mut engine_status_h);
        });
    }

    // ---------- Zeitkontrolle ----------
    {
        let state = state.clone();
        movetime_input.set_callback(move |i| {
            if let Ok(ms) = i.value().parse::<u32>() {
                state.borrow_mut().movetime_ms = ms.max(1);
            }
        });
    }
    {
        let state = state.clone();
        infinite_check.set_callback(move |c| {
            state.borrow_mut().infinite = c.is_checked();
        });
    }
    {
        let state = state.clone();
        stop_btn.set_callback(move |_| {
            if let Some(engine) = state.borrow_mut().engine.as_mut() {
                let _ = engine.stop_search();
            }
        });
    }

    // ---------- Farbwahl ----------
    {
        let state = state.clone();
        color_choice.set_callback(move |c| {
            let mut st = state.borrow_mut();
            st.human_is_white = c.value() == 0;
        });
    }

    // ---------- Automatisch zuletzt genutzte Engine laden ----------
    if let Some(last_path) = load_last_engine_path() {
        if std::path::Path::new(&last_path).exists() {
            let state = state.clone();
            let mut engine_status_h = engine_status.clone();
            try_load_engine(&last_path, &state, &mut engine_status_h);
        }
    }

    // ---------- Neue Partie ----------
    {
        let state = state.clone();
        let mut move_list_h = move_list.clone();
        let mut board_frame_h = board_frame.clone();
        let mut eval_bar_h = eval_bar.clone();
        new_game_btn.set_callback(move |_| {
            let mut st = state.borrow_mut();
            st.board.reset();
            st.game_over = false;
            move_list_h.clear();
            eval_bar_h.set_value(0.0);
            eval_bar_h.set_label("0.00");
            if let Some(engine) = st.engine.as_mut() {
                let _ = engine.new_game();
            }
            let human_white = st.human_is_white;
            if !human_white {
                request_engine_move(&mut st);
            }
            drop(st);
            board_frame_h.redraw();
        });
    }

    // ---------- Brett drehen ----------
    {
        let state = state.clone();
        let mut board_frame_h = board_frame.clone();
        flip_btn.set_callback(move |_| {
            state.borrow_mut().board.flipped ^= true;
            board_frame_h.redraw();
        });
    }

    // ---------- Turnier-/SPRT-Fenster öffnen ----------
    {
        tournament_btn.set_callback(move |_| {
            emblium::tournament_window::open_tournament_window();
        });
    }

    // ---------- Engine-Events pollen (alle 50ms) ----------
    {
        let state = state.clone();
        let mut move_list_h = move_list.clone();
        let mut board_frame_h = board_frame.clone();
        let mut eval_bar_h = eval_bar.clone();
        let mut engine_status_h = engine_status.clone();
        let mut output_buffer_h = output_buffer.clone();

        app::add_timeout3(0.05, move |handle| {
            {
                let mut st = state.borrow_mut();
                let mut events: Vec<EngineEvent> = Vec::new();
                if let Some(engine) = st.engine.as_ref() {
                    while let Ok(ev) = engine.events.try_recv() {
                        events.push(ev);
                    }
                }
                for ev in events {
                    match ev {
                        EngineEvent::RawLine(line) => {
                            output_buffer_h.append(&line);
                            output_buffer_h.append("\n");
                        }
                        EngineEvent::Ready => {
                            engine_status_h.set_label("Engine bereit");
                        }
                        EngineEvent::IdName(name) => {
                            engine_status_h.set_label(&format!("Geladen: {name}"));
                        }
                        EngineEvent::IdAuthor(_) => {}
                        EngineEvent::Info(info) => {
                            if let Some(cp) = info.score_cp {
                                let mut pawns = cp as f64 / 100.0;
                                if st.board.position.turn() == shakmaty::Color::Black {
                                    pawns = -pawns;
                                }
                                eval_bar_h.set_value((pawns * 100.0).clamp(-1000.0, 1000.0));
                                eval_bar_h.set_label(&format!("{pawns:+.2}"));
                            } else if let Some(mate) = info.score_mate {
                                eval_bar_h.set_label(&format!("Matt in {}", mate.abs()));
                            }
                        }
                        EngineEvent::BestMove { best, ponder: _ } => {
                            if best != "(none)" {
                                if let Err(e) = st.board.push_uci(&best) {
                                    output_buffer_h.append(&format!("[GUI-Fehler] {e}\n"));
                                }
                                refresh_move_list(&st.board, &mut move_list_h);
                            }
                            st.waiting_for_engine = false;
                            check_game_over(&mut st);
                            board_frame_h.redraw();
                        }
                        EngineEvent::Crashed(msg) => {
                            engine_status_h.set_label("Engine abgestürzt");
                            output_buffer_h.append(&format!("[Absturz] {msg}\n"));
                        }
                    }
                }
            }
            app::repeat_timeout3(0.05, handle);
        });
    }

    // Referenzen am Leben halten, die nur in Closures verwendet werden
    let _keep = (
        eval_label, move_list_label, output_label, color_choice.clone(),
        time_row_label, ms_label,
    );

    app.run().unwrap();
}

fn style_accent_button(btn: &mut Button) {
    btn.set_frame(FrameType::FlatBox);
    btn.set_color(fltk::enums::Color::from_rgb(0x5B, 0x8D, 0xEF));
    btn.set_label_color(fltk::enums::Color::from_rgb(0xFF, 0xFF, 0xFF));
    btn.clear_visible_focus();
}

fn style_flat_button(btn: &mut Button) {
    btn.set_frame(FrameType::FlatBox);
    btn.set_color(fltk::enums::Color::from_rgb(0x3A, 0x3A, 0x4A));
    btn.set_label_color(fltk::enums::Color::from_rgb(0xE8, 0xE8, 0xEC));
    btn.clear_visible_focus();
}

fn style_warning_button(btn: &mut Button) {
    btn.set_frame(FrameType::FlatBox);
    btn.set_color(fltk::enums::Color::from_rgb(0xC0, 0x5C, 0x4A));
    btn.set_label_color(fltk::enums::Color::from_rgb(0xFF, 0xFF, 0xFF));
    btn.clear_visible_focus();
}

fn try_load_engine(path: &str, state: &Rc<RefCell<AppState>>, engine_status: &mut Frame) {
    match UciEngine::start(path) {
        Ok(engine) => {
            state.borrow_mut().engine = Some(engine);
            engine_status.set_label(&format!("Lade: {path} ..."));
            save_last_engine_path(path);
        }
        Err(e) => {
            dialog::alert_default(&format!("Engine konnte nicht gestartet werden:\n{e}"));
        }
    }
}

fn refresh_move_list(board: &BoardState, list: &mut HoldBrowser) {
    list.clear();
    let uci_moves = board.uci_history();
    let mut line = String::new();
    for (i, mv) in uci_moves.iter().enumerate() {
        if i % 2 == 0 {
            line = format!("{}. {}", i / 2 + 1, mv);
        } else {
            line = format!("{line}   {mv}");
            list.add(&line);
            line.clear();
        }
    }
    if !line.is_empty() {
        list.add(&line);
    }
    list.bottom_line(list.size());
}

fn check_game_over(st: &mut AppState) {
    if let Some(msg) = st.board.game_over_message() {
        st.game_over = true;
        dialog::message_default(&msg);
    }
}

fn request_engine_move(st: &mut AppState) {
    let Some(engine) = st.engine.as_mut() else { return };
    st.waiting_for_engine = true;
    let moves = st.board.uci_history();
    let _ = engine.set_position(&moves);
    if st.infinite {
        let _ = engine.go_infinite();
    } else {
        let _ = engine.go_movetime(st.movetime_ms);
    }
}
