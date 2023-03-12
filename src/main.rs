use std::{env::{self, current_dir}, path::{Path, Component}};

use compiler::compile;
use interpreter::interpret;
use parser::parse;

mod compiler;
mod interpreter;
mod parser;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    let args: Vec<String> = env::args().collect();

    match args.get(1) {
        Some(s) => match s.as_str() {
            "run" => {
                let a = args.get(2);
                match a {
                    Some(a) => run(a),
                    None => {
                        println!("ERROR: expected file");
                    }
                }
            }
            "help" => {
                println!("Available commands:\n");
                println!("help          this command");
                println!("run <file>    interpret clink file");
            }
            "build" => {
                let a = args.get(2);
                match a {
                    Some(a) => do_compile(a),
                    None => {
                        println!("ERROR: expected file");
                    }
                }
            }
            _ => {
                println!("ERROR: unknown command");
                println!("HINT:  type 'clink help' for commands");
            }
        }
        _ => {
            println!("ERROR: expected command");
            println!("HINT:  type 'clink help' for commands");
        }
    }
}

fn run(file: &String) {

    let path = Path::new(file).to_path_buf();

    let program = parse(&path);

    if let Err(e) = program {
        println!("{}", e);
        return;
    }

    let mut vec_path = Vec::new();
    for component in path.with_extension("").components() {
        if let Component::Normal(x) = component {
            match x.to_str() {
                Some(x) => vec_path.push(x.to_string()),
                None => {
                    println!("ERROR: string read error");
                    return;
                },
            }
        }
    }
    vec_path.push("_".to_string());

    let result = interpret(&(program.unwrap()), vec_path);

    if let Err(e) = result {
        println!("{}", e);
        return;
    }
}

fn do_compile(file: &String) {
    let path = Path::new(file).to_path_buf();

    let program = parse(&path);

    if let Err(e) = program {
        println!("{}", e);
        return;
    }

    let mut vec_path = Vec::new();
    for component in path.with_extension("").components() {
        if let Component::Normal(x) = component {
            match x.to_str() {
                Some(x) => vec_path.push(x.to_string()),
                None => {
                    println!("ERROR: string read error");
                    return;
                },
            }
        }
    }
    vec_path.push("_".to_string());

    compile(current_dir().unwrap().file_name().unwrap().to_str().unwrap(), program.unwrap(), vec_path);
}