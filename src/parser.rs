use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs,
    iter::Peekable,
    str::Chars, path::{PathBuf, Path, Component},
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
    Left(Box<AST>),
    Right(Box<AST>),
    Print(Box<AST>),
    Read(Box<AST>),
    Split(Box<AST>, Box<AST>, Box<AST>),
    Apply(Box<AST>, Box<AST>),
    Id(Box<AST>, Vec<String>),
    Param,
}

#[derive(Debug)]
pub enum ParseError {
    FileNotFound(String),
    ExpectedPackageName,
    CannotDefineFunctionOutsidePackage(Vec<String>),
    FunctionDefinedTwice(String),
    UnknownFunction(Vec<String>),
    AmbiguousReference(Vec<String>),
    UnknownAssociativity,
    StringReadError
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::FileNotFound(p) => write!(f, "ERROR: file `{}` not found", p),
            ParseError::ExpectedPackageName => write!(f, "ERROR: expected package name"),
            ParseError::CannotDefineFunctionOutsidePackage(id) => {
                write!(f, "ERROR: cannot define function `{}` outside package", id.join("."))
            }
            ParseError::UnknownFunction(path) => {
                write!(f, "ERROR: unknown function {}", path.join("."))
            }
            ParseError::AmbiguousReference(id) => write!(f, "ERROR: ambiguous reference `{}`", id.join(".")),
            ParseError::UnknownAssociativity => write!(f, "ERROR: unknown associativity of `:`"),
            ParseError::StringReadError => write!(f, "ERROR: string read error"),
            ParseError::FunctionDefinedTwice(id) => write!(f, "ERROR: function `{}` defined twice", id),
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

pub fn parse(dir: &Path) -> Result<HashMap<Vec<String>, AST>, ParseError> {
    let mut queued = HashMap::new();

    // scan all known functions and return functions from dir
    let mut functions = scan_funcs(dir, &mut queued)?;

    // scan through and find all known references
    scan_for_references(&mut functions, &queued)?;

    let mut referenced_funcs = HashMap::new();

    for function in functions {
        let v = queued.remove(&function).unwrap();
        referenced_funcs.insert(
            function,
            parse_functions(parse_colon(parse_brackets(v)?)?),
        );
    }

    Ok(referenced_funcs)
}

fn scan_funcs(
    dir: &Path,
    private_funcs: &mut HashMap<Vec<String>, Vec<Token>>,
) -> Result<HashSet<Vec<String>>, ParseError> {
    let input = fs::read_to_string(dir).map_err(|_| {
        let s = dir.to_str();
        match s {
            Some(s) => ParseError::FileNotFound(s.to_string()),
            None => ParseError::StringReadError,
        }
    })?;

    let mut vec_path = Vec::new();
    for component in dir.with_extension("").components() {
        if let Component::Normal(x) = component {
            let st = x.to_str().ok_or(ParseError::StringReadError)?;
            vec_path.push(st.to_string())
            
        }
    }

    // find all functions and packages

    let mut tokenised = tokenise(input.as_str())?.into_iter();
    let mut defining = false;

    let mut packages = HashSet::new();
    let mut functions = HashMap::new();

    let mut function_tokens = Vec::new();
    let mut function_name = String::new();

    let mut function_names = HashSet::new();

    loop {
        match tokenised.next() {
            None => break,
            Some(token) => {
                if defining {
                    if let Token::Semicolon = token {
                        defining = false;
                        if function_names.contains(&function_name) {
                            return Err(ParseError::FunctionDefinedTwice(function_name))
                        }
                        function_names.insert(function_name.clone());
                        functions.insert(function_name.clone(), function_tokens);
                        function_tokens = Vec::new();
                    } else {
                        function_tokens.push(token);
                    }
                } else {
                    if let Token::Bang = token {
                        let pkg = tokenised.next();
                        if let Some(Token::Id(pkg_name)) = pkg {
                            packages.insert(pkg_name);
                        } else {
                            return Err(ParseError::ExpectedPackageName);
                        }
                    } else if let Token::Id(id) = token {
                        if id.len() != 1 {
                            return Err(ParseError::CannotDefineFunctionOutsidePackage(id));
                        }
                        let mut mid = id;
                        function_name = mid.pop().unwrap();
                        defining = true;
                    }
                }
            }
        }
    }
    if defining {
        if function_names.contains(&function_name) {
            return Err(ParseError::FunctionDefinedTwice(function_name))
        }
        function_names.insert(function_name.clone());
        functions.insert(function_name.clone(), function_tokens);
    }

    // get knowledge of functions in all referenced packages recursively

    for package in &packages {
        let mut path = PathBuf::new();
        for sub in package {
            path.push(sub);
        }
        scan_funcs(&path.with_extension("clink"), private_funcs)?;
    }

    // for each function, get references, and search referenced packages for matches

    let mut referenced_funcs = HashMap::new();

    for (function_name, function_tokens) in functions {
        let mut new_tokens = Vec::new();
        for token in function_tokens {
            if let Token::Id(id) = token {
                if id.len() == 1 && function_names.contains(id.last().unwrap()) {
                    let mut d = vec_path.clone();
                    d.append(&mut id.clone());
                    new_tokens.push(Token::Id(d.clone()));
                } else {
                    let mut found = None;
                    for package in &packages {
                        let mut d = package.clone();
                        d.append(&mut id.clone());
                        if private_funcs.contains_key(&d) {
                            if let Some(_) = found {
                                return Err(ParseError::AmbiguousReference(id));
                            } else {
                                found = Some(d);
                            }
                        }
                    }
                    if let None = found {
                        return Err(ParseError::UnknownFunction(id));
                    }
                    new_tokens.push(Token::Id(found.unwrap()))
                }
            } else {
                new_tokens.push(token)
            }
        }

        referenced_funcs.insert(function_name, new_tokens);
    }

    // add all functions to list of known functions

    for (function_name, function_tokens) in referenced_funcs {
        let mut d = vec_path.clone();
        d.push(function_name.clone());
        private_funcs.insert(d, function_tokens);
    }

    let mut output = HashSet::new();

    for function_name in function_names {
        let mut d = vec_path.clone();
        d.push(function_name.clone());
        output.insert(d);
    }

    Ok(output)
}

fn scan_for_references(
    names: &mut HashSet<Vec<String>>,
    funcs: &HashMap<Vec<String>, Vec<Token>>,
) -> Result<(), ParseError> {
    let mut ids_to_check = HashSet::new();
    for name in names.iter() {
        let function = funcs.get(name).unwrap();
        for token in function {
            if let Token::Id(id) = token {
                if !names.contains(id) {
                    ids_to_check.insert(id);
                }
            }
        }
    }

    if ids_to_check.is_empty() {
        Ok(())
    } else {
        for id in ids_to_check {
            names.insert(id.clone());
        }
        scan_for_references(names, funcs)
    }
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

fn parse_functions(func: Vec<Token>) -> AST {
    let mut current = AST::Param;
    for token in func.into_iter().rev() {
        match token {
            Token::Bracket(ts) => {
                if let AST::Param = current {
                    current = parse_functions(ts)
                } else {
                    current = AST::Apply(Box::new(current), Box::new(parse_functions(ts)))
                }
            }
            Token::Bang => current = AST::Left(Box::new(current)),
            Token::Question => current = AST::Right(Box::new(current)),
            Token::At => current = AST::Read(Box::new(current)),
            Token::Hash => current = AST::Print(Box::new(current)),
            Token::Split(l, r) => {
                current = AST::Split(
                    Box::new(current),
                    Box::new(parse_functions(l)),
                    Box::new(parse_functions(r)),
                )
            }
            Token::Id(id) => current = AST::Id(Box::new(current), id),
            _ => {}
        }
    }

    current
}
