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

    // match &mut parsed {
    //     Err(e) => println!("{}", e),
    //     Ok(parsed) => loop {
    //         use std::io::{stdin, stdout, Write};
    //         let mut s = String::new();
    //         print!("> ");
    //         stdout().flush().unwrap();
    //         stdin().read_line(&mut s).unwrap();
    //         if let Some('\n') = s.chars().next_back() {
    //             s.pop();
    //         }
    //         if let Some('\r') = s.chars().next_back() {
    //             s.pop();
    //         }
    //         let parsed_expr = parser::parse_statement(s.as_str(), parsed);
    //         match parsed_expr {
    //             Ok(parsed_expr) => {
    //                 let result = interpret(&parsed, &parsed_expr);
    //                 match result {
    //                     Ok(r) => println!("< {}", r),
    //                     Err(e) => println!("{}", e),
    //                 }
    //             }
    //             Err(err) => {
    //                 println!("{}", err)
    //             }
    //         }
    //     },
    // }
}
