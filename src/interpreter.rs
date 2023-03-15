use std::{collections::HashMap, fmt::Display};

use crate::parser::{self, AST};

#[derive(Debug)]
pub enum RuntimeError {
    NoSuchFunction(Vec<String>),
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::NoSuchFunction(s) => write!(f, "ERROR: no such function {}", s.join(".")),
        }
    }
}

pub fn interpret(
    program: &HashMap<Vec<String>, Vec<AST>>,
    entry: Vec<String>,
) -> Result<(), RuntimeError> {
    let mut result = Vec::new();
    do_ast(
        program,
        &mut result,
        program
            .get(&entry)
            .ok_or(RuntimeError::NoSuchFunction(entry))?,
    )?;
    Ok(())
}

fn do_ast(
    program: &HashMap<Vec<String>, Vec<AST>>,
    param: &mut Vec<bool>,
    asts: &Vec<AST>,
) -> Result<(), RuntimeError> {
    for ast in asts {
        match ast {
            AST::Left => {
                param.push(true);
            }
            AST::Right => {
                param.push(false);
            }
            parser::AST::Split(l, r) => {
                if param.pop().unwrap_or(false) {
                    do_ast(program, param, l)?;
                } else {
                    do_ast(program, param, r)?;
                }
            }
            parser::AST::Bracketed(f) => {
                do_ast(program, param, f)?;
            }
            parser::AST::Id(id) => {
                let f = program.get(id).unwrap();
                do_ast(program, param, f)?;
            }
            parser::AST::Print => {
                let mut total: u8 = 0;
                for _ in 0..8 {
                    total *= 2;
                    if param.pop().unwrap_or(false) {
                        total += 1;
                    }
                }
                print!("{}", char::from(total));
            }
            parser::AST::Read => {
                let mut code: u8 = read_char().try_into().unwrap();
                for _ in 0..8 {
                    if code % 2 == 0 {
                        param.push(false);
                    } else {
                        param.push(true);
                    }
                    code /= 2;
                }
            }
        }
    }
    Ok(())
}

fn read_char() -> char {
    use std::io::stdin;
    let mut s = String::new();
    stdin().read_line(&mut s).unwrap();
    if let Some('\n') = s.chars().next_back() {
        s.pop();
    }
    if let Some('\r') = s.chars().next_back() {
        s.pop();
    }
    s.chars().next().unwrap()
}
