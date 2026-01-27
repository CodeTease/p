use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while, take_while1},
    character::complete::{char, multispace0, multispace1, satisfy, digit1, one_of},
    combinator::{map, peek, opt, cut},
    multi::{many0, many1, fold_many0, separated_list0},
    sequence::{delimited, pair, preceded},
    IResult,
};
use nom::error::Error;
use crate::pas::ast::{CommandExpr, RedirectMode, Arg, ArgPart};
use crate::pas::context::ShellContext;

struct ParserContext<'a> {
    _ctx: &'a ShellContext,
}

pub fn parse_command_line(input: &str, ctx: &ShellContext) -> anyhow::Result<CommandExpr> {
    let pctx = ParserContext { _ctx: ctx };
    match parse_sequence(input, &pctx) {
        Ok((rem, expr)) => {
            let (rem, _) = multispace0::<&str, nom::error::Error<&str>>(rem).unwrap_or((rem, ""));
            if !rem.is_empty() {
                anyhow::bail!("Unexpected input remaining: {}", rem);
            }
            Ok(expr)
        },
        Err(e) => anyhow::bail!("Parse error: {}", e),
    }
}

// 0. Sequence: ;
fn parse_sequence<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, list) = separated_list0(
        delimited(multispace0, char(';'), multispace0), 
        |i| parse_logic(i, pctx)
    )(input)?;
    
    if list.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify)));
    }
    
    let mut iter = list.into_iter();
    let first = iter.next().unwrap();
    
    let res = iter.fold(first, |acc, next| {
        CommandExpr::Sequence(Box::new(acc), Box::new(next))
    });
    
    Ok((input, res))
}

// 1. Logic: &&, ||
fn parse_logic<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, init) = parse_pipe(input, pctx)?;

    fold_many0(
        pair(
            delimited(multispace0, alt((tag("&&"), tag("||"))), multispace0),
            |i| parse_pipe(i, pctx)
        ),
        move || init.clone(),
        |acc, (op, next)| {
            match op {
                "&&" => CommandExpr::And(Box::new(acc), Box::new(next)),
                "||" => CommandExpr::Or(Box::new(acc), Box::new(next)),
                _ => unreachable!(),
            }
        }
    )(input)
}

// 2. Pipe: |
fn parse_pipe<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, init) = parse_redirect(input, pctx)?;

    fold_many0(
        preceded(
            delimited(multispace0, char('|'), multispace0),
            |i| parse_redirect(i, pctx)
        ),
        move || init.clone(),
        |acc, next| {
            CommandExpr::Pipe {
                left: Box::new(acc),
                right: Box::new(next),
            }
        }
    )(input)
}

// 3. Redirect: >, >>, <, 2>&1
fn parse_redirect<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, cmd) = parse_atomic(input, pctx)?;
    
    let (input, redirects) = many0(|i| parse_redirect_entry(i, pctx))(input)?;
    
    let res = redirects.into_iter().rev().fold(cmd, |acc, (mode, target, source_fd)| {
        CommandExpr::Redirect {
            cmd: Box::new(acc),
            target,
            mode,
            source_fd,
        }
    });
    
    Ok((input, res))
}

fn parse_redirect_entry<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, (RedirectMode, Arg, i32)> {
    let (input, _) = multispace0(input)?;
    let (input, fd_str) = opt(digit1)(input)?;
    let source_fd = fd_str.map(|s: &str| s.parse::<i32>().unwrap()).unwrap_or(-1);

    alt((
        // 2>&1
        map(preceded(tag(">&"), cut(digit1)), move |target_fd: &str| {
             let src = if source_fd == -1 { 1 } else { source_fd };
             (RedirectMode::MergeStderrToStdout, Arg(vec![ArgPart::Literal(target_fd.to_string())]), src)
        }),
        // >>
        map(preceded(tag(">>"), cut(preceded(multispace0, |i| parse_token(i, pctx)))), move |target| {
             let src = if source_fd == -1 { 1 } else { source_fd };
             (RedirectMode::Append, target, src)
        }),
        // >
        map(preceded(tag(">"), cut(preceded(multispace0, |i| parse_token(i, pctx)))), move |target| {
             let src = if source_fd == -1 { 1 } else { source_fd };
             (RedirectMode::Overwrite, target, src)
        }),
        // <
        map(preceded(tag("<"), cut(preceded(multispace0, |i| parse_token(i, pctx)))), move |target| {
             let src = if source_fd == -1 { 0 } else { source_fd };
             (RedirectMode::Input, target, src)
        }),
    ))(input)
}

// 4. Atomic: If, While, Subshell, Simple/Assignment
fn parse_atomic<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    alt((
        |i| parse_if(i, pctx),
        |i| parse_while(i, pctx),
        |i| parse_subshell(i, pctx),
        |i| parse_simple(i, pctx)
    ))(input)
}

fn optional_separator<'a>(input: &'a str) -> IResult<&'a str, ()> {
    let (input, _) = multispace0(input)?;
    if let Ok((rem, _)) = char::<_, nom::error::Error<&str>>(';')(input) {
        let (rem, _) = multispace0(rem)?;
        Ok((rem, ()))
    } else {
        Ok((input, ()))
    }
}

// If
fn parse_if<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, _) = tag("if")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, cond) = parse_sequence(input, pctx)?;
    let (input, _) = optional_separator(input)?;
    
    let (input, _) = tag("then")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, then_branch) = parse_sequence(input, pctx)?;
    let (input, _) = optional_separator(input)?;
    
    let (input, else_branch) = match tag::<_, _, nom::error::Error<&str>>("else")(input) {
        Ok((rem, _)) => {
            let (rem, _) = multispace1(rem)?;
            let (rem, branch) = parse_sequence(rem, pctx)?;
            let (rem, _) = optional_separator(rem)?;
            (rem, Some(Box::new(branch)))
        },
        Err(_) => (input, None)
    };
    
    let (input, _) = tag("fi")(input)?;
    
    Ok((input, CommandExpr::If {
        cond: Box::new(cond),
        then_branch: Box::new(then_branch),
        else_branch,
    }))
}

// While
fn parse_while<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, _) = tag("while")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, cond) = parse_sequence(input, pctx)?;
    let (input, _) = optional_separator(input)?;
    
    let (input, _) = tag("do")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, body) = parse_sequence(input, pctx)?;
    let (input, _) = optional_separator(input)?;
    
    let (input, _) = tag("done")(input)?;
    
    Ok((input, CommandExpr::While {
        cond: Box::new(cond),
        body: Box::new(body),
    }))
}

// Subshell
fn parse_subshell<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, _) = char('(')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, cmd) = parse_sequence(input, pctx)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;
    
    Ok((input, CommandExpr::Subshell(Box::new(cmd))))
}

// Simple or Assignment
fn parse_simple<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, program) = parse_token(input, pctx)?;
    
    if let Some(s) = arg_as_static_keyword(&program) {
        if is_keyword(&s) {
            return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)));
        }
    }

    let (input, args) = many0(preceded(multispace1, |i| parse_token(i, pctx)))(input)?;
    
    if args.is_empty() {
        if let Some((key, value)) = check_assignment(&program) {
            return Ok((input, CommandExpr::Assignment { key, value }));
        }
    }
    
    Ok((input, CommandExpr::Simple { program, args }))
}

fn arg_as_static_keyword(arg: &Arg) -> Option<String> {
    if arg.0.len() == 1 {
        if let ArgPart::Literal(ref s) = arg.0[0] {
            return Some(s.clone());
        }
    }
    None
}

fn check_assignment(arg: &Arg) -> Option<(String, Arg)> {
    if let Some(first) = arg.0.first() {
        if let ArgPart::Literal(s) = first { // Removed 'ref'
            if let Some(idx) = s.find('=') {
                if idx > 0 {
                    let key = s[..idx].to_string();
                    if key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        let val_prefix = s[idx+1..].to_string();
                        let mut value_parts = Vec::new();
                        if !val_prefix.is_empty() {
                            value_parts.push(ArgPart::Literal(val_prefix));
                        }
                        value_parts.extend_from_slice(&arg.0[1..]);
                        return Some((key, Arg(value_parts)));
                    }
                }
            }
        }
    }
    None
}

fn is_keyword(s: &str) -> bool {
    matches!(s, "if" | "then" | "else" | "fi" | "while" | "do" | "done")
}

fn parse_token<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, Arg> {
    let (input, parts_list) = many1(alt((
        parse_single_quoted,
        |i| parse_double_quoted(i, pctx),
        parse_escaped_char_part,
        |i| parse_variable(i, pctx),
        parse_unquoted_text_part
    )))(input)?;
    
    let mut combined = Vec::new();
    for p in parts_list {
        combined.extend(p);
    }
    Ok((input, Arg(combined)))
}

fn parse_single_quoted(input: &str) -> IResult<&str, Vec<ArgPart>> {
    let (input, s) = delimited(
        char('\''),
        map(take_while(|c| c != '\''), |s: &str| s.to_string()),
        char('\'')
    )(input)?;
    Ok((input, vec![ArgPart::Literal(s)]))
}

fn parse_double_quoted<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, Vec<ArgPart>> {
    let (input, _) = char('"')(input)?;
    let (input, parts_list) = many0(alt((
        parse_escaped_char_part,
        |i| parse_variable(i, pctx),
        map(is_not("\"$\\"), |s: &str| vec![ArgPart::Literal(s.to_string())])
    )))(input)?;
    let (input, _) = char('"')(input)?;
    
    let mut combined = Vec::new();
    for p in parts_list {
        combined.extend(p);
    }
    Ok((input, combined))
}

fn parse_escaped_char_part(input: &str) -> IResult<&str, Vec<ArgPart>> {
    let (input, _) = char('\\')(input)?;
    let (input, c) = satisfy(|_| true)(input)?;
    Ok((input, vec![ArgPart::Literal(c.to_string())]))
}

fn parse_variable<'a>(input: &'a str, _pctx: &ParserContext) -> IResult<&'a str, Vec<ArgPart>> {
    let (input, _) = char('$')(input)?;
    
    if let Ok((rem, _)) = char::<_, nom::error::Error<&str>>('{')(input) {
        let (rem, name) = take_while1(|c: char| c != '}')(rem)?;
        let (rem, _) = char('}')(rem)?;
        return Ok((rem, vec![ArgPart::Variable(name.to_string())]));
    }
    
    if let Ok((rem, _)) = char::<_, nom::error::Error<&str>>('?')(input) {
        return Ok((rem, vec![ArgPart::Variable("?".to_string())]));
    }
    
    let (input, name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    Ok((input, vec![ArgPart::Variable(name.to_string())]))
}

fn parse_unquoted_text_part(input: &str) -> IResult<&str, Vec<ArgPart>> {
    // Stop if we see start of redirect (digit followed by > or <)
    if let Ok((_, _)) = peek(pair(digit1::<&str, Error<&str>>, one_of::<&str, &str, Error<&str>>("><")))(input) {
         return Err(nom::Err::Error(Error::new(input, nom::error::ErrorKind::Tag)));
    }

    take_while1(|c: char| !c.is_whitespace() && !is_quote(c) && c != '$' && c != '\\' && !is_operator_char(c))(input)
        .map(|(next, res)| (next, vec![ArgPart::Literal(res.to_string())]))
}

fn is_operator_char(c: char) -> bool {
    "|&><;()".contains(c)
}

fn is_quote(c: char) -> bool {
    c == '\'' || c == '"'
}
