use std::io;
use std::process;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use clap::Parser;
use opomodoro::{App, Config};

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

fn main () -> io::Result<()> {
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
            
        })
        .as_secs();
    
    let break_time = cli.break_time
        .parse::<humantime::Duration>()
        .unwrap_or_else(|err| {
            eprintln!("Issue parsing break time argument: {err}");
            process::exit(1);
            
        })
        .as_secs();
    
    let cycles = cli.num_cycles;

    let late: bool = cli.late;

    let config = Config { 
        work_time: Duration::from_secs(work_time), 
        break_time: Duration::from_secs(break_time), 
        cycles, 
        late 
    };

    let mut app = App::new(config, &running.as_ref());
    ratatui::run(|terminal| 
        App::run(&mut app, terminal))?;
    println!("Exiting...");
    std::thread::sleep(Duration::from_millis(500));
    println!("See you next time!");
    Ok(())
}