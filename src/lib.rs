mod ui;
use std::thread;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};
use crossterm::event::{
    KeyCode,
    KeyModifiers, 
    read, 
    poll,
};
use ratatui::{DefaultTerminal, Frame};

#[derive(Debug)]
struct Phase<'a> {
    kind: &'a str,
    duration: Duration,
}

impl<'a> Phase<'a> {
   fn build(
    kind: &'a str,
    duration: Duration,
    ) -> Phase<'a> {
        Phase { kind, duration }
    }
}

#[derive(Debug)]
pub struct Config {
    pub work_time: Duration,
    pub break_time: Duration,
    pub cycles: u32,
    pub late: bool,
}

#[derive(Debug, Clone, Copy)]
enum TimerState {
    Running { end: Instant },
    Paused { remaining: Duration },
}

impl TimerState {
        fn toggle_pause(&mut self, now: Instant) {
            *self = match *self {
                TimerState::Running { end } => {
                    let remaining = end.saturating_duration_since(now);
                    TimerState::Paused { remaining }
                }
                TimerState::Paused { remaining } => {
                    TimerState::Running { end: now + remaining }
                }
            };
        }

        fn remaining(&self, now: Instant) -> Duration {
            match *self {
                TimerState::Running { end } => end.saturating_duration_since(now),
                TimerState::Paused { remaining } => remaining,
            }
    }
}

#[derive(Debug)]
enum Action {
    Toggle,
    Skip,
    Quit,
    None,
}

#[derive(Debug, PartialEq)]
enum EndState {
    None,
    Completed,
    Skipped,
    Erred,
    Quit,
}


#[derive(Debug)]
pub struct App<'a> {
    current_cycle: u32,
    num_cycles: u32,
    work_time: Duration,
    break_time: Duration,
    phase: Phase<'a>,
    timer_state: TimerState,
    end_state: EndState, 
    running: &'a AtomicBool,
    remaining: Duration,
    late: bool,
}

impl<'a> App<'a> {
    pub fn run (&mut self, 
        terminal: &mut DefaultTerminal
    ) -> io::Result<()> {
         {
            while self.end_state != EndState::Quit {
                if ! self.running.load(Ordering::Relaxed) {
                    self.end_state = EndState::Quit;
                    break;
                }
                let now = Instant::now();
                self.remaining = self.timer_state.remaining(now);
                let action = self.handle_input();
                self.apply_action(action, now);
                self.update(now);
                terminal.draw(|frame| self.draw(frame))?;
            };
        }
        Ok(())
    }

    pub fn new (
    config: Config,
    running: &'a AtomicBool,
    ) -> App<'a> {
        let current_cycle: u32 = 1;
        let num_cycles = config.cycles;
        let work_time = config.work_time;
        let break_time = config.break_time;
        let phase = Phase::build("Work", work_time);
        let timer_state = TimerState::Running 
            { end: Instant::now() + work_time  };
        let end_state = EndState::None;
        let late = config.late;
        let remaining = work_time;
        App {
            current_cycle, 
            num_cycles, 
            work_time, 
            break_time,
            phase,
            timer_state,
            end_state,
            running,
            remaining,
            late, 
        }
    }

    fn draw(&self, frame: &mut Frame) {
        ui::render(frame, self);
    }

    fn handle_input(&mut self) -> Action {
        match poll(Duration::from_millis(100)) {
            Ok(true) => {
                let read_event = match read() {
                    Ok(ev) => ev,
                    Err(e) => {
                        eprintln!("Error reading event: {e}");
                        self.end_state = EndState::Erred;
                        return Action::Quit;
                    }
                };

                if let Some(key) = read_event.as_key_press_event() {
                    match key.code {
                        KeyCode::Char('p') => {
                            return Action::Toggle;
                        }
                        KeyCode::Char('s') => {
                            return Action::Skip;
                        }
                        KeyCode::Char('q') => {
                            return Action::Quit;
                        }
                        KeyCode::Char('c') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                return Action::Quit; 
                            }
                            return Action::None;   
                        }
                        _ => {
                            return Action::None;
                        }
                    }
                }
                Action::None
            }
            Ok(false) => {
                Action::None
            } 
            Err(e) => {
                eprintln!("Error polling events: {e}");
                self.end_state = EndState::Erred;
                Action::Quit
            }
        }
    }

    fn apply_action(&mut self, action: Action, now: Instant) {
        match action {
            Action::Toggle => {
                self.timer_state.toggle_pause(now);    
            }
            Action::Skip => {
                self.end_state = EndState::Skipped;
            }
            Action::Quit => {
                self.running.store(false, Ordering::Relaxed);
                self.end_state = EndState::Quit;
            }
            _ => {},    
        }
    } 

    fn update(&mut self, now: Instant) {
        if matches!(self.timer_state, TimerState::Running { .. }) 
            && self.remaining == Duration::ZERO {
            self.end_state = EndState::Completed;
        }
        match self.end_state {
            EndState::Completed => {
                thread::sleep(Duration::from_millis(300));
                print!("\x07");
                io::stdout().flush().unwrap();
                if self.phase.kind == "Work" {
                    if self.current_cycle == self.num_cycles && ! self.late {
                        self.end_state = EndState::Quit;
                    } else {
                        self.end_state = EndState::None;
                        self.phase = Phase { kind: "Break", duration: self.break_time };
                        self.timer_state = TimerState::Running { end: now + self.break_time };
                    }
                } else {
                    if self.current_cycle == self.num_cycles {
                        self.end_state = EndState::Quit;
                    } else {
                        self.end_state = EndState::None;
                        self.phase = Phase { kind: "Work", duration: self.work_time };
                        self.timer_state = TimerState::Running { end: now + self.work_time };
                        self.current_cycle += 1;
                    }
                }
            }
            EndState::Skipped => {
                thread::sleep(Duration::from_millis(300));
                if self.phase.kind == "Work" {
                    if self.current_cycle == self.num_cycles && ! self.late {
                        self.end_state = EndState::Quit;
                    } else {
                        self.end_state = EndState::None;
                        self.phase = Phase { kind: "Break", duration: self.break_time };
                        self.timer_state = TimerState::Running { end: now + self.break_time };
                    }
                } else {
                    if self.current_cycle == self.num_cycles {
                        self.end_state = EndState::Quit;
                    } else {
                        self.end_state = EndState::None;
                        self.phase = Phase { kind: "Work", duration: self.work_time };
                        self.timer_state = TimerState::Running { end: now + self.work_time };
                        self.current_cycle += 1;
                    }
                }
            }
            _ => {},
        }
    }
}
