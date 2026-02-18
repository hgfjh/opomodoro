use std::{process, thread};
use std::time::{Duration, Instant};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use clap::Parser;
use crossterm::event::{KeyCode, KeyModifiers, poll, read};
use opomodoro;

#[derive(Parser)]
#[command(name = "Opomodoro")]
#[command(version = "1.0")]
#[command(about = "Pomodoro in the command line.", long_about = None)]
struct Cli {
    #[arg(long = "work")]
    work_time: String,
    #[arg(long = "break")]
    break_time: String,
    #[arg(long = "cycles")]
    num_cycles: u32,
    #[arg(short, long)]
    late: bool,
}

enum EndState {
    Completed,
    Skipped,
    Erred,
    Quit,
}

struct Phase<'a> {
    kind: &'a str,
    duration: Duration,
}

impl<'a> Phase<'a> {
   fn build(
    kind: &'a str, 
    duration: Duration
    ) -> Phase<'a> {
        Phase { kind, duration }
    }

   fn run(
    kind: &'a str, 
    duration: Duration, 
    cycle: u32, 
    num_cycles: u32,
    running: &AtomicBool
    ) -> EndState {
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

        fn is_running(&self) -> bool {
            matches!(self, TimerState::Running { .. })
        }
    }

    let mut state = TimerState::Running {
        end: Instant::now() + duration,
    };

    let mut last_shown_secs: Option<u64> = None;
    let mut force_redraw = true;

    let mut endstate = EndState::Completed;

    loop {
        if !running.load(Ordering::Relaxed) {
            endstate = EndState::Quit;
            break;
        }

        match poll(Duration::from_millis(100)) {
            Ok(true) => {
                let read_event = match read() {
                    Ok(ev) => ev,
                    Err(e) => {
                        eprintln!("Error reading event: {e}");
                        endstate = EndState::Erred;
                        break;
                    }
                };

                if let Some(key) = read_event.as_key_press_event() {
                    match key.code {
                        KeyCode::Char('p') => {
                            state.toggle_pause(Instant::now());
                            force_redraw = true;
                        }
                        KeyCode::Char('s') => {
                            print!("\r Cycle {cycle}/{num_cycles} {kind} 00:00    ");
                            io::stdout().flush().unwrap();
                            print!("\r {kind} skipped!          ");
                            io::stdout().flush().unwrap();
                            thread::sleep(Duration::from_millis(300));
                            endstate = EndState::Skipped;
                            break;
                        }
                        KeyCode::Char('q') => {
                            endstate = EndState::Quit;
                            running.store(false, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('c') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                endstate = EndState::Quit;
                                running.store(false, Ordering::Relaxed);
                                break; 
                            }   
                        }
                        _ => {},
                    }
                }
            }
            Ok(false) => {} 
            Err(e) => {
                eprintln!("Error polling events: {e}");
                endstate = EndState::Erred;
                break;
            }
        }

        let now = Instant::now();
        let remaining = state.remaining(now);
        let remaining_secs = remaining.as_secs();

        if state.is_running() && remaining_secs == 0 {
            print!("\r {kind} done!    ");
            io::stdout().flush().unwrap();
            print!("\x07");
            io::stdout().flush().unwrap();
            break;
        }

        if force_redraw || last_shown_secs != Some(remaining_secs) {
            let mm = remaining_secs / 60;
            let ss = remaining_secs % 60;

            let label = if state.is_running() { kind } else { "Paused" };

            print!("\r Cycle {cycle}/{num_cycles} {label} {mm:02}:{ss:02}    ");
            io::stdout().flush().unwrap();

            last_shown_secs = Some(remaining_secs);
            force_redraw = false;
        }
    }
    endstate
   } 
}
fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    }).expect("Error setting Ctrl-C handler");

    let cli = Cli::parse();

    let work_time = cli.work_time
        .parse::<humantime::Duration>()
        .unwrap_or_else(|err| {
            eprintln!("Issue parsing work time argument: {err}");
            process::exit(1);
            
        });
    
    let break_time = cli.break_time
        .parse::<humantime::Duration>()
        .unwrap_or_else(|err| {
            eprintln!("Issue parsing break time argument: {err}");
            process::exit(1);
            
        });
    
    let num_cycles = cli.num_cycles;

    let std_work: Duration = work_time.into();

    let std_break: Duration = break_time.into();

    let mut end_with_quit = false;

    for i in 1..num_cycles+1 {
        let work_phase = Phase::build("Work", std_work);
        let break_phase = Phase::build("Break", std_break);

        if let EndState::Quit = Phase::run(
            work_phase.kind, 
            work_phase.duration, 
            i, 
            num_cycles,
            &running
        ) {
            end_with_quit = true;
            break;
        }

        if i == num_cycles && ! cli.late {
            break;
        }

        if let EndState::Quit = Phase::run(
            break_phase.kind, 
            break_phase.duration, 
            i, 
            num_cycles,
            &running
        ) {
            end_with_quit = true;
            break;
        }
        
    }

    if ! end_with_quit {
        print!("\r Good job on having the discipline to see your work through! Do it again tomorrow.");
    } else {
        println!("\r Quitting...          ");
        thread::sleep(Duration::from_millis(300));
        println!("Try to stick to it next time.");
    }
    
}
