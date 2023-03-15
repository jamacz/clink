use std::{
    collections::{HashMap, HashSet},
    env::current_dir,
    fmt::Display,
    fs,
    iter::Peekable,
    path::Path,
    str::Chars,
};

#[derive(Debug)]
pub enum Token {
    Bang,
    Question,
    Colon,
    Semicolon,
    At,
    Hash,
    LBracket,
    RBracket,
    Bracket(Vec<Token>),
    Split(Vec<Token>, Vec<Token>),
    Id(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum AST {
    Left,
    Right,
    Print,
    Read,
    Split(Vec<AST>, Vec<AST>),
    Bracketed(Vec<AST>),
    Id(Vec<String>),
}

#[derive(Debug)]
pub enum ParseError {
    FileNotFound(String),
    ExpectedPackageName,
    CannotDefineFunctionOutsidePackage(Vec<String>),
    FunctionDefinedTwice(String),
    UnknownFunction(Vec<String>),
    UnknownPackage(Vec<String>),
    AmbiguousReference(Vec<String>),
    UnknownAssociativity,
    OSStringConversionError,
    CannotFindCurrentDir,
    ErrorReadingDirectory,
    CannotGetMetadata,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::FileNotFound(p) => write!(f, "ERROR: file `{}` not found", p),
            ParseError::ExpectedPackageName => write!(f, "ERROR: expected package name"),
            ParseError::CannotDefineFunctionOutsidePackage(id) => {
                write!(
                    f,
                    "ERROR: cannot define function `{}` outside package",
                    id.join(".")
                )
            }
            ParseError::UnknownFunction(path) => {
                write!(f, "ERROR: unknown function {}", path.join("."))
            }
            ParseError::AmbiguousReference(id) => {
                write!(f, "ERROR: ambiguous reference `{}`", id.join("."))
            }
            ParseError::UnknownAssociativity => write!(f, "ERROR: unknown associativity of `:`"),
            ParseError::FunctionDefinedTwice(id) => {
                write!(f, "ERROR: function `{}` defined twice", id)
            }
            ParseError::UnknownPackage(path) => {
                write!(f, "ERROR: unknown package {}", path.join("."))
            }
            ParseError::CannotFindCurrentDir => write!(f, "ERROR: cannot find current directory"),
            ParseError::ErrorReadingDirectory => write!(f, "ERROR: cannot read directory"),
            ParseError::OSStringConversionError => write!(f, "ERROR: OSStr converstion error"),
            ParseError::CannotGetMetadata => write!(f, "ERROR: cannot get metadata"),
        }
    }
}

pub fn tokenise(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut rest = input.chars().peekable();
    loop {
        let token;
        (token, rest) = next_token(rest)?;
        match token {
            None => break,
            Some(token) => tokens.push(token),
        }
    }
    Ok(tokens)
}

fn next_token(i: Peekable<Chars>) -> Result<(Option<Token>, Peekable<Chars>), ParseError> {
    let mut input = i;
    while input.peek().map_or(false, |x| x.is_whitespace()) {
        input.next();
    }
    match input.peek() {
        None => Ok((None, input)),
        Some(char) => match char {
            '!' => {
                input.next();
                Ok((Some(Token::Bang), input))
            }
            '?' => {
                input.next();
                Ok((Some(Token::Question), input))
            }
            ':' => {
                input.next();
                Ok((Some(Token::Colon), input))
            }
            '@' => {
                input.next();
                Ok((Some(Token::At), input))
            }
            '#' => {
                input.next();
                Ok((Some(Token::Hash), input))
            }
            ';' => {
                input.next();
                Ok((Some(Token::Semicolon), input))
            }
            '(' => {
                input.next();
                Ok((Some(Token::LBracket), input))
            }
            ')' => {
                input.next();
                Ok((Some(Token::RBracket), input))
            }
            _ => {
                let mut id = String::new();
                loop {
                    match input.peek() {
                        None => break,
                        Some(char) => {
                            match char {
                                '!' | '?' | ':' | '@' | '#' | ';' | '(' | ')' => break,
                                _ => {}
                            }
                            if !char.is_whitespace() {
                                id.push(input.next().unwrap());
                            } else {
                                break;
                            }
                        }
                    }
                }
                Ok((
                    Some(Token::Id(id.split(".").map(|x| x.to_string()).collect())),
                    input,
                ))
            }
        },
    }
}

// -------------------------------------------------

pub fn parse(main_func: &mut Vec<String>) -> Result<HashMap<Vec<String>, Vec<AST>>, ParseError> {
    let directory = current_dir().map_err(|_| ParseError::CannotFindCurrentDir)?;
    let mut functions = HashMap::new();
    let mut packages = HashSet::new();
    let mut imported_packages = HashSet::new();
    let mut imports = HashMap::new();

    scan_dir(
        &directory,
        Vec::new(),
        &mut functions,
        &mut packages,
        &mut imported_packages,
        &mut imports,
    )?;

    for pkg in imported_packages {
        if !packages.contains(&pkg) {
            return Err(ParseError::UnknownPackage(pkg))
        }
    }

    let mut func_defs = HashMap::new();

    parse_funcs(main_func, &mut func_defs, &mut functions, &mut imports)?;

    dbg!(&func_defs);

    Ok(func_defs)
}

fn scan_dir(
    dir: &Path,
    pkg: Vec<String>,
    functions: &mut HashMap<Vec<String>, Vec<Token>>,
    packages: &mut HashSet<Vec<String>>,
    imported_packages: &mut HashSet<Vec<String>>,
    imports: &mut HashMap<Vec<String>, HashSet<Vec<String>>>,
) -> Result<(), ParseError> {
    for file in dir
        .read_dir()
        .map_err(|_| ParseError::ErrorReadingDirectory)?
    {
        if let Ok(file) = file {
            let mut file_name = pkg.clone();
            file_name.push(
                file.path()
                    .with_extension("")
                    .file_name()
                    .ok_or(ParseError::CannotGetMetadata)?
                    .to_str()
                    .ok_or(ParseError::OSStringConversionError)?
                    .to_string(),
            );
            packages.insert(file_name.clone());
            if file.metadata().unwrap().is_dir() {
                scan_dir(
                    file.path().as_path(),
                    file_name,
                    functions,
                    packages,
                    imported_packages,
                    imports,
                )?;
            } else if let Some(t) = file.path().extension() {
                //check if clink file
                if t == "clink" {
                    let content = fs::read_to_string(file.path()).map_err(|_| {
                        match file.path().to_str() {
                            Some(th) => ParseError::FileNotFound(th.to_string()),
                            None => ParseError::OSStringConversionError,
                        }
                    })?;

                    let tokenised = tokenise(content.as_str())?;

                    let mut defining = false;
                    let mut importing = false;
                    let mut current_func = Vec::new();
                    let mut current_func_name = String::new();

                    for token in tokenised {
                        if importing {
                            if let Token::Id(id) = token {
                                if let None = imports.get(&file_name) {
                                    imports.insert(file_name.clone(), HashSet::new());
                                }
                                imported_packages.insert(id.clone());
                                imports.get_mut(&file_name).unwrap().insert(id);
                            } else {
                                return Err(ParseError::ExpectedPackageName);
                            }
                            importing = false;
                        } else if defining {
                            if let Token::Semicolon = token {
                                let mut f_n = file_name.clone();
                                f_n.push(current_func_name);
                                if functions.contains_key(&f_n) {
                                    return Err(ParseError::FunctionDefinedTwice(f_n.join(".")));
                                }
                                functions.insert(f_n, current_func);
                                current_func = Vec::new();
                                current_func_name = String::new();
                                defining = false;
                            } else {
                                current_func.push(token);
                            }
                        } else {
                            if let Token::Bang = token {
                                importing = true;
                            } else if let Token::Id(id) = token {
                                if id.len() != 1 {
                                    return Err(ParseError::CannotDefineFunctionOutsidePackage(id));
                                }
                                current_func_name = id.first().unwrap().clone();
                                defining = true;
                            }
                        }
                    }

                    if defining {
                        let mut f_n = file_name.clone();
                        f_n.push(current_func_name);
                        if functions.contains_key(&f_n) {
                            return Err(ParseError::FunctionDefinedTwice(f_n.join(".")));
                        }
                        functions.insert(f_n, current_func);
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_funcs(
    current: &Vec<String>,
    func_defs: &mut HashMap<Vec<String>, Vec<AST>>,
    functions: &mut HashMap<Vec<String>, Vec<Token>>,
    imports: &mut HashMap<Vec<String>, HashSet<Vec<String>>>,
) -> Result<(), ParseError> {
    let mut dirn = current.clone();
    dirn.pop();

    let f = functions.remove(current);
    if let None = f {
        return Ok(());
    }
    let f = f.unwrap();

    let mut to_parse = Vec::new();

    let mut new_f = Vec::new();

    for token in f {
        if let Token::Id(id) = token {
            let mut found = None;
            if current == &id || functions.contains_key(&id) || func_defs.contains_key(&id) {
                found = Some(id.clone());
            } else {
                let mut ds = Vec::new();
                for d in &dirn {
                    ds.push(d.clone());
                    let mut m = ds.clone();
                    m.append(&mut id.clone());
                    if current == &m || functions.contains_key(&m) || func_defs.contains_key(&m) {
                        if let None = found {
                            found = Some(m.clone());
                        } else {
                            return Err(ParseError::AmbiguousReference(id));
                        }
                    }
                }
            }

            if let None = found {
                for import in imports.get(&dirn).unwrap() {
                    let mut ds = Vec::new();
                    for d in import {
                        ds.push(d.clone());
                        let mut m = ds.clone();
                        m.append(&mut id.clone());
                        if current == &m || functions.contains_key(&m) || func_defs.contains_key(&m)
                        {
                            if let None = found {
                                found = Some(m.clone());
                            } else {
                                return Err(ParseError::AmbiguousReference(id));
                            }
                        }
                    }
                }
            }

            match found {
                Some(x) => {
                    to_parse.push(x.clone());
                    new_f.push(Token::Id(x))
                }
                None => return Err(ParseError::UnknownFunction(id.clone())),
            }
        } else {
            new_f.push(token);
        }
    }

    let p_f = parse_functions(parse_colon(parse_brackets(new_f)?)?);
    func_defs.insert(current.clone(), p_f);

    for mut t_p in to_parse {
        dbg!(&t_p);
        parse_funcs(&mut t_p, func_defs, functions, imports)?;
    }

    Ok(())
}

fn parse_brackets(func: Vec<Token>) -> Result<Vec<Token>, ParseError> {
    parse_brackets_each(0, &mut func.into_iter().peekable())
}

fn parse_brackets_each(
    level: i32,
    func: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    loop {
        match func.peek() {
            Some(Token::LBracket) => {
                func.next();
                tokens.push(Token::Bracket(parse_brackets_each(level + 1, func)?));
            }
            Some(Token::RBracket) => {
                func.next();
                return Ok(tokens);
            }
            Some(_) => {
                let t = func.next();
                tokens.push(t.unwrap())
            }
            None => return Ok(tokens),
        }
    }
}

fn parse_colon(func: Vec<Token>) -> Result<Vec<Token>, ParseError> {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut split = false;
    for token in func {
        match token {
            Token::Colon => {
                if split {
                    return Err(ParseError::UnknownAssociativity);
                } else {
                    split = true;
                }
            }
            Token::Bracket(contents) => {
                if split {
                    right.push(Token::Bracket(parse_colon(contents)?));
                } else {
                    left.push(Token::Bracket(parse_colon(contents)?));
                }
            }
            t => {
                if split {
                    right.push(t);
                } else {
                    left.push(t);
                }
            }
        }
    }
    if split {
        let mut s = Vec::new();
        s.push(Token::Split(left, right));
        return Ok(s);
    } else {
        return Ok(left);
    }
}

fn parse_functions(func: Vec<Token>) -> Vec<AST> {
    let mut current = Vec::new();
    for token in func.into_iter().rev() {
        match token {
            Token::Bracket(ts) => {
                if current.is_empty() {
                    current = parse_functions(ts)
                } else {
                    current.push(AST::Bracketed(parse_functions(ts)))
                }
            }
            Token::Bang => current.push(AST::Left),
            Token::Question => current.push(AST::Right),
            Token::At => current.push(AST::Read),
            Token::Hash => current.push(AST::Print),
            Token::Split(l, r) => current.push(AST::Split(parse_functions(l), parse_functions(r))),
            Token::Id(id) => current.push(AST::Id(id)),
            _ => {}
        }
    }

    current
}
