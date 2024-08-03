use crossterm::{
    cursor,
    event::{self, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, ClearType},
    ExecutableCommand,
};
use regex::Regex;
use std::{
    env,
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

const HISTORY_FILE: &str = "history.txt";

fn load_history() -> Arc<Mutex<Vec<String>>> {
    let path = Path::new(HISTORY_FILE);
    let history = if path.exists() {
        let file = File::open(path).expect("Unable to open history file");
        let reader = BufReader::new(file);
        reader.lines().filter_map(Result::ok).collect()
    } else {
        Vec::new()
    };
    Arc::new(Mutex::new(history))
}

fn save_to_history(command: &str) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(HISTORY_FILE)
        .expect("Unable to open history file");
    writeln!(file, "{}", command).expect("Unable to write to history file");
}

fn search_history(history: &Arc<Mutex<Vec<String>>>, query: &str) -> Vec<String> {
    let history = history.lock().unwrap();
    let regex = Regex::new(&format!("(?i){}", regex::escape(query))).unwrap();
    history
        .iter()
        .filter(|entry| regex.is_match(entry))
        .cloned()
        .collect()
}

fn main() {
    let history = load_history();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        // Save command to history
        save_to_history(input);

        // Handle command input
        let mut commands = input.split(" | ").peekable();
        let mut previous_command = None;

        while let Some(command) = commands.next() {
            let mut parts = command.trim().split_whitespace();
            let command = parts.next().unwrap();
            let args = parts;

            match command {
                "cd" => {
                    let new_dir = args.peekable().peek().map_or("/", |x| *x);
                    let root = Path::new(new_dir);
                    if let Err(e) = env::set_current_dir(&root) {
                        eprintln!("{}", e);
                    }
                    previous_command = None;
                }
                "exit" => return,
                command => {
                    let stdin = previous_command.map_or(Stdio::inherit(), |output: Child| {
                        Stdio::from(output.stdout.unwrap())
                    });

                    let stdout = if commands.peek().is_some() {
                        Stdio::piped()
                    } else {
                        Stdio::inherit()
                    };

                    let output = Command::new(command)
                        .args(args)
                        .stdin(stdin)
                        .stdout(stdout)
                        .spawn();

                    match output {
                        Ok(output) => {
                            previous_command = Some(output);
                        }
                        Err(e) => {
                            previous_command = None;
                            eprintln!("{}", e);
                        }
                    };
                }
            }
        }

        if let Some(mut final_command) = previous_command {
            final_command.wait().unwrap();
        }

        // Handle search functionality
        let mut search_input = String::new();
        print!("Search history: ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut search_input).unwrap();
        let search_query = search_input.trim();

        let matches = search_history(&history, search_query);
        if !matches.is_empty() {
            println!("Matching commands:");
            for entry in matches {
                println!("{}", entry);
            }
        } else {
            println!("No matching commands found.");
        }
    }
}
