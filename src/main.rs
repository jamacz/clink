use std::{env::{self}, path::{Path, Component}};

use interpreter::interpret;

mod interpreter;
mod parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = Path::new(&args[1]).to_path_buf();

    let program = parser::parse(&path);

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
