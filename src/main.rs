use std::collections::HashMap;
use std::fs;
use std::process;

use clap::Parser;
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "tm",
    version = "0.1.0",
    author = "Frank Buss",
    about = "A Turing Machine simulator that loads a machine from JSON"
)]
struct Cli {
    /// if present, only print every 100k steps (faster)
    #[arg(long)]
    fast: bool,

    /// the JSON file describing the Turing Machine
    filename: String,
}

// ANSI codes for inverse video and reset
const INVERSE: &str = "\u{001B}[7m";
const RESET: &str = "\u{001B}[0m";

fn main() {
    // parse command line and read file
    let cli = Cli::parse();
    let text = match fs::read_to_string(&cli.filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file {}: {}", cli.filename, e);
            process::exit(1);
        }
    };
    let tm: Value = match json5::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing JSON5 in {}: {}", cli.filename, e);
            process::exit(1);
        }
    };

    // mandatory: "blank" & "initial"
    let blank = match tm.get("blank").and_then(|v| v.as_str()) {
        Some(b) => b,
        None => {
            eprintln!("Error: JSON missing mandatory field 'blank'.");
            process::exit(1);
        }
    };
    let initial = match tm.get("initial").and_then(|v| v.as_str()) {
        Some(i) => i,
        None => {
            eprintln!("Error: JSON missing mandatory field 'initial'.");
            process::exit(1);
        }
    };

    // optional fields
    let name = tm.get("name").and_then(|v| v.as_str());
    let desc = tm.get("description").and_then(|v| v.as_str());
    let author = tm.get("author").and_then(|v| v.as_str());
    let date = tm.get("date").and_then(|v| v.as_str());
    let link = tm.get("link").and_then(|v| v.as_str());
    let paper = tm.get("paper").and_then(|v| v.as_str());

    let tape_str = tm.get("tape").and_then(|v| v.as_str()).unwrap_or("");
    let mut position = tm.get("position").and_then(|v| v.as_i64()).unwrap_or(0);

    // build transitions
    let mut transitions: HashMap<(String, String), (String, String, String)> = HashMap::new();
    if let Some(rules) = tm.get("transitions").and_then(|v| v.as_array()) {
        for rule in rules {
            if let Some(arr) = rule.as_array() {
                if arr.len() == 5 {
                    let old_state = to_str(&arr[0]);
                    let old_symbol = to_str(&arr[1]);
                    let new_symbol = to_str(&arr[2]);
                    let direction = to_str(&arr[3]).to_uppercase();
                    let new_state = to_str(&arr[4]);
                    transitions.insert((old_state, old_symbol), (new_symbol, direction, new_state));
                }
            }
        }
    }

    // create initial tape
    let mut tape: Vec<String> = tape_str.chars().map(|c| c.to_string()).collect();
    if tape.is_empty() {
        tape.push(blank.to_string());
    }

    //  position fix
    while position < 0 {
        tape.insert(0, blank.to_string());
        position += 1;
    }
    while (position as usize) >= tape.len() {
        tape.push(blank.to_string());
    }

    // print summary
    println!("\nStarting Turing machine");
    if let Some(n) = name {
        println!("Name: {}", n);
    }
    if let Some(d) = desc {
        println!("Description: {}", d);
    }
    if let Some(a) = author {
        println!("Author: {}", a);
    }
    if let Some(dte) = date {
        println!("Date: {}", dte);
    }
    if let Some(ln) = link {
        println!("Link: {}", ln);
    }
    if let Some(pp) = paper {
        println!("Paper: {}", pp);
    }

    println!(
        "(blank='{}', initial='{}', position={})",
        blank, initial, position
    );
    println!();

    // run the Turing machine
    let fast_mode = cli.fast;
    let mut step = 0_i64;
    let mut state = initial.to_string();

    loop {
        let do_print = if fast_mode { step % 100_000 == 0 } else { true };

        if do_print {
            let pos_u = position as usize;
            let mut joined_tape = String::new();
            for (i, sym) in tape.iter().enumerate() {
                if i == pos_u {
                    joined_tape.push_str(INVERSE);
                    joined_tape.push_str(sym);
                    joined_tape.push_str(RESET);
                } else {
                    joined_tape.push_str(sym);
                }
            }
            println!(
                "Step {:6}  State={:<8} Pos={:<5} Tape={}",
                step, state, position, joined_tape
            );
        }

        // current symbol
        let pos_u = position as usize;
        let current_symbol = &tape[pos_u];

        // find rule
        let key = (state.clone(), current_symbol.clone());
        let (mut new_symbol, direction, mut new_state) =
            if let Some(&(ref ns, ref dir, ref st)) = transitions.get(&key) {
                (ns.clone(), dir.clone(), st.clone())
            } else if let Some(&(ref ns, ref dir, ref st)) =
                transitions.get(&(state.clone(), "*".to_string()))
            {
                let w = if ns == "*" {
                    current_symbol.clone()
                } else {
                    ns.clone()
                };
                (w, dir.clone(), st.clone())
            } else {
                println!(
                    "\nHalted: no rule for state={} and symbol='{}'",
                    state, current_symbol
                );
                break;
            };

        if new_symbol == "*" {
            new_symbol = current_symbol.clone();
        }
        if new_state == "*" {
            new_state = state.clone();
        }
        tape[pos_u] = new_symbol;
        state = new_state;

        match direction.as_str() {
            "L" => {
                position -= 1;
                if position < 0 {
                    tape.insert(0, blank.to_string());
                    position = 0;
                }
            }
            "R" => {
                position += 1;
                while (position as usize) >= tape.len() {
                    tape.push(blank.to_string());
                }
            }
            "S" | _ => { /* do nothing */ }
        }

        step += 1;
    }

    // final tape
    let mut final_tape = tape.join("");
    while final_tape.starts_with(blank) {
        final_tape.drain(..blank.len());
    }
    while final_tape.ends_with(blank) {
        let cut = final_tape.len() - blank.len();
        final_tape.drain(cut..);
    }

    println!("\nFinal tape: {}", final_tape);
    println!("Steps: {}", step);

    // print histogram
    let mut histogram: HashMap<String, i64> = HashMap::new();
    for sym in tape {
        *histogram.entry(sym).or_insert(0) += 1;
    }
    println!("Histogram:");
    for (sym, count) in histogram {
        println!("  symbol: {}, count: {}", sym, count);
    }
}

/// Converts a serde_json::Value to a string, removing surrounding quotes
fn to_str(val: &Value) -> String {
    val.to_string().trim_matches('"').to_owned()
}
