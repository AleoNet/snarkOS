use std::io;
use std::sync::mpsc;
use std::{thread, time};

use tracing::level_filters::LevelFilter;
// use log::*;

use termion::{
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen,
};

use tui::backend::Backend;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Tabs};
use tui::Frame;
use tui::Terminal;

use crate::console::*;

struct App {
    states: Vec<TuiWidgetState>,
    tabs: Vec<String>,
    selected_tab: usize,
    opt_info_cnt: Option<u16>,
}

#[derive(Debug)]
enum AppEvent {
    Termion(termion::event::Event),
    LoopCnt(Option<u16>),
}

// fn demo_application(tx: mpsc::Sender<AppEvent>) {
//     let one_second = time::Duration::from_millis(1_000);
//     let mut lp_cnt = (1..=100).into_iter();
//     loop {
//         trace!(target:"DEMO", "Sleep one second");
//         thread::sleep(one_second);
//         trace!(target:"DEMO", "Issue log entry for each level");
//         error!(target:"error", "an error");
//         warn!(target:"warn", "a warning");
//         trace!(target:"trace", "a trace");
//         debug!(target:"debug", "a debug");
//         info!(target:"info", "an info");
//         tx.send(AppEvent::LoopCnt(lp_cnt.next())).unwrap();
//     }
// }

pub fn start_console() -> std::result::Result<(), std::io::Error> {
    // init_logger(LevelFilter::Trace).unwrap();
    // set_default_level(LevelFilter::Trace);
    info!(target:"DEMO", "Start demo");

    let backend = {
        let stdout = io::stdout().into_raw_mode().unwrap();
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        TermionBackend::new(stdout)
    };

    let mut terminal = Terminal::new(backend).unwrap();
    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();

    // Use an mpsc::channel to combine stdin events with app events
    let (tx, rx) = mpsc::channel();
    let tx_event = tx.clone();
    thread::spawn({
        let f = {
            let stdin = io::stdin();
            move || {
                for c in stdin.events() {
                    trace!(target:"DEMO", "Stdin event received {:?}", c);
                    tx_event.send(AppEvent::Termion(c.unwrap())).unwrap();
                }
            }
        };
        f
    });
    // thread::spawn(move || {
    //     demo_application(tx);
    // });

    let mut app = App {
        states: vec![],
        tabs: vec!["V1".into(), "V2".into(), "V3".into(), "V4".into()],
        selected_tab: 0,
        opt_info_cnt: None,
    };

    // Here is the main loop
    for evt in rx {
        trace!(target: "New event", "{:?}",evt);
        let opt_state = if app.selected_tab < app.states.len() {
            Some(&mut app.states[app.selected_tab])
        } else {
            None
        };
        match evt {
            AppEvent::Termion(evt) => {
                use termion::event::{Event, Key};
                match evt {
                    Event::Key(Key::Char('q')) => break,
                    Event::Key(Key::Char('\t')) => {
                        // tab
                        let sel = app.selected_tab;
                        let sel_tab = if sel + 1 < app.tabs.len() { sel + 1 } else { 0 };
                        app.selected_tab = sel_tab;
                    }
                    evt => {
                        if let Some(state) = opt_state {
                            match evt {
                                Event::Key(Key::Char(' ')) => {
                                    state.transition(&TuiWidgetEvent::SpaceKey);
                                }
                                Event::Key(Key::Esc) => {
                                    state.transition(&TuiWidgetEvent::EscapeKey);
                                }
                                Event::Key(Key::PageUp) => {
                                    state.transition(&TuiWidgetEvent::PrevPageKey);
                                }
                                Event::Key(Key::PageDown) => {
                                    state.transition(&TuiWidgetEvent::NextPageKey);
                                }
                                Event::Key(Key::Up) => {
                                    state.transition(&TuiWidgetEvent::UpKey);
                                }
                                Event::Key(Key::Down) => {
                                    state.transition(&TuiWidgetEvent::DownKey);
                                }
                                Event::Key(Key::Left) => {
                                    state.transition(&TuiWidgetEvent::LeftKey);
                                }
                                Event::Key(Key::Right) => {
                                    state.transition(&TuiWidgetEvent::RightKey);
                                }
                                Event::Key(Key::Char('+')) => {
                                    state.transition(&TuiWidgetEvent::PlusKey);
                                }
                                Event::Key(Key::Char('-')) => {
                                    state.transition(&TuiWidgetEvent::MinusKey);
                                }
                                Event::Key(Key::Char('h')) => {
                                    state.transition(&TuiWidgetEvent::HideKey);
                                }
                                Event::Key(Key::Char('f')) => {
                                    state.transition(&TuiWidgetEvent::FocusKey);
                                }
                                Event::Key(Key::Char('q')) => {
                                    panic!("Exiting like this for now")
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
            AppEvent::LoopCnt(opt_cnt) => {
                app.opt_info_cnt = opt_cnt;
            }
        }
        terminal.draw(|mut f| {
            let size = f.size();
            draw_frame(&mut f, size, &mut app);
        })?;
    }
    terminal.show_cursor().unwrap();
    terminal.clear().unwrap();

    Ok(())
}

fn draw_frame<B: Backend>(t: &mut Frame<B>, size: Rect, app: &mut App) {
    let tabs: Vec<tui::text::Spans> = vec!["V1".into(), "V2".into(), "V3".into(), "V4".into()];
    let sel = app.selected_tab;

    if app.states.len() <= sel {
        app.states.push(TuiWidgetState::new());
    }

    let block = Block::default().borders(Borders::ALL);
    let inner_area = block.inner(size);
    t.render_widget(block, size);

    let mut constraints = vec![
        Constraint::Length(3),
        Constraint::Percentage(50),
        Constraint::Min(3),
    ];
    if app.opt_info_cnt.is_some() {
        constraints.push(Constraint::Length(3));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    let tabs = Tabs::new(tabs)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .select(sel);
    t.render_widget(tabs, chunks[0]);

    let tui_sm = TuiLoggerSmartWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_debug(Style::default().fg(Color::Green))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_trace(Style::default().fg(Color::Magenta))
        .style_info(Style::default().fg(Color::Cyan))
        .state(&mut app.states[sel]);
    t.render_widget(tui_sm, chunks[1]);
    let tui_w: TuiLoggerWidget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title("Independent Tui Logger View")
                .border_style(Style::default().fg(Color::White).bg(Color::Black))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black));
    t.render_widget(tui_w, chunks[2]);
    if let Some(percent) = app.opt_info_cnt {
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::ITALIC),
            )
            .percent(percent);
        t.render_widget(gauge, chunks[3]);
    }
}
