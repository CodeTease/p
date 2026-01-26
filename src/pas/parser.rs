use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while, take_while1},
    character::complete::{char, multispace0, multispace1, satisfy},
    combinator::map,
    multi::{many0, many1, fold_many0},
    sequence::{delimited, pair, preceded},
    IResult,
};
use crate::pas::ast::{CommandExpr, RedirectMode};
use crate::pas::context::ShellContext;
use std::path::MAIN_SEPARATOR;

// Helper to pass context
struct ParserContext<'a> {
    ctx: &'a ShellContext,
}

pub fn parse_command_line(input: &str, ctx: &ShellContext) -> anyhow::Result<CommandExpr> {
    let pctx = ParserContext { ctx };
    match parse_logic(input, &pctx) {
        Ok((rem, expr)) => {
            // Ensure we consumed everything (except maybe whitespace)
            let (rem, _) = multispace0::<&str, nom::error::Error<&str>>(rem).unwrap_or((rem, ""));
            if !rem.is_empty() {
                anyhow::bail!("Unexpected input remaining: {}", rem);
            }
            Ok(expr)
        },
        Err(e) => anyhow::bail!("Parse error: {}", e),
    }
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

// 3. Redirect: >, >>, <
fn parse_redirect<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    let (input, cmd) = parse_simple(input, pctx)?;
    
    fold_many0(
        pair(
            delimited(multispace0, alt((tag(">>"), tag(">"), tag("<"))), multispace0),
            |i| parse_token(i, pctx)
        ),
        move || cmd.clone(),
        |acc, (op, target)| {
            let mode = match op {
                ">" => RedirectMode::Overwrite,
                ">>" => RedirectMode::Append,
                "<" => RedirectMode::Input,
                _ => unreachable!(),
            };
            CommandExpr::Redirect {
                cmd: Box::new(acc),
                target,
                mode,
            }
        }
    )(input)
}

// 4. Simple: program + args
fn parse_simple<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, CommandExpr> {
    // A simple command is a sequence of tokens separated by spaces.
    // It must have at least one token (the program).
    
    let (input, program) = parse_token(input, pctx)?;
    let (input, args) = many0(preceded(multispace1, |i| parse_token(i, pctx)))(input)?;
    
    // Check if we didn't consume a token that looks like an operator?
    // parse_token should not consume operators if they are unquoted.
    // Actually, `parse_token` needs to stop at operators `|`, `&`, `>`, `<`.
    
    Ok((input, CommandExpr::Simple { program, args }))
}

// 5. Token: Expansion, Quotes
fn parse_token<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, String> {
    // A token is a mix of:
    // - Unquoted chars (excluding whitespace and operators)
    // - Quoted strings (single/double)
    // - Escaped chars
    // All concatenated.
    
    let (input, parts) = many1(alt((
        parse_single_quoted,
        |i| parse_double_quoted(i, pctx),
        parse_escaped_char,
        |i| parse_variable(i, pctx),
        parse_unquoted_text
    )))(input)?;
    
    let token = parts.concat();
    Ok((input, normalize_path(token)))
}

fn is_operator_char(c: char) -> bool {
    "|&><;".contains(c)
}

fn parse_unquoted_text(input: &str) -> IResult<&str, String> {
    // Read until whitespace, quote, $, \, or operator
    take_while1(|c: char| !c.is_whitespace() && !is_quote(c) && c != '$' && c != '\\' && !is_operator_char(c))(input)
        .map(|(next, res)| (next, res.to_string()))
}

fn is_quote(c: char) -> bool {
    c == '\'' || c == '"'
}

fn parse_escaped_char(input: &str) -> IResult<&str, String> {
    let (input, _) = char('\\')(input)?;
    let (input, c) = satisfy(|_| true)(input)?; // Take any char
    Ok((input, c.to_string()))
}

fn parse_single_quoted(input: &str) -> IResult<&str, String> {
    delimited(
        char('\''),
        map(take_while(|c| c != '\''), |s: &str| s.to_string()),
        char('\'')
    )(input)
}

fn parse_double_quoted<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, String> {
    let (input, _) = char('"')(input)?;
    let (input, parts) = many0(alt((
        parse_escaped_char,
        |i| parse_variable(i, pctx),
        map(is_not("\"$\\"), |s: &str| s.to_string())
    )))(input)?;
    let (input, _) = char('"')(input)?;
    Ok((input, parts.concat()))
}

fn parse_variable<'a>(input: &'a str, pctx: &ParserContext) -> IResult<&'a str, String> {
    let (input, _) = char('$')(input)?;
    
    // Check for brace
    if let Ok((rem, _)) = char::<_, nom::error::Error<&str>>('{')(input) {
        let (rem, name) = take_while1(|c: char| c != '}')(rem)?;
        let (rem, _) = char('}')(rem)?;
        let val = pctx.ctx.env.get(name).cloned().unwrap_or_default();
        return Ok((rem, val));
    }
    
    // Check for special vars
    if let Ok((rem, _)) = char::<_, nom::error::Error<&str>>('?')(input) {
        return Ok((rem, pctx.ctx.exit_code.to_string()));
    }
    
    // Normal var name: alphanumeric + _
    let (input, name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let val = pctx.ctx.env.get(name).cloned().unwrap_or_default();
    Ok((input, val))
}

fn normalize_path(token: String) -> String {
    if cfg!(windows) {
        if token.contains('/') {
            return token.replace('/', &MAIN_SEPARATOR.to_string());
        }
    }
    token
}
