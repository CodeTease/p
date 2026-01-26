use crate::pas::context::ShellContext;
use anyhow::Result;
use std::path::MAIN_SEPARATOR;

pub fn parse_command(cmd_str: &str, ctx: &ShellContext) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current_token = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    
    // Used to track if we actually parsed something into the current token
    // so we can distinguish between empty string (from "") vs nothing (whitespace)
    let mut token_started = false; 
    
    // We need to handle the case where a token is explicitly empty e.g. ""
    // My previous logic relied on !is_empty(), which fails for "".
    // Let's refine the logic.
    // If we see a quote, we mark token_started = true.
    // If we see a char, token_started = true.
    // If we see space and token_started, we push.

    let mut chars = cmd_str.chars().peekable();

    while let Some(c) = chars.next() {
        if escaped {
            current_token.push(c);
            escaped = false;
            token_started = true;
            continue;
        }

        if c == '\\' {
            if in_single_quote {
                current_token.push(c);
                token_started = true;
            } else {
                escaped = true;
                token_started = true; // The backslash starts the token even if next is space (escaped space)
            }
            continue;
        }

        if c == '\'' {
            if in_double_quote {
                current_token.push(c);
                token_started = true;
            } else {
                in_single_quote = !in_single_quote;
                token_started = true; // Quote characters imply a token exists (even if empty)
            }
            continue;
        }

        if c == '"' {
            if in_single_quote {
                current_token.push(c);
                token_started = true;
            } else {
                in_double_quote = !in_double_quote;
                token_started = true;
            }
            continue;
        }

        if c == '$' && !in_single_quote {
            // Expansion
            token_started = true; 
            
            let mut var_name = String::new();
            
            if let Some(&'{') = chars.peek() {
                chars.next(); // consume {
                while let Some(&vc) = chars.peek() {
                    if vc == '}' {
                        chars.next(); // consume }
                        break;
                    }
                    var_name.push(chars.next().unwrap());
                }
            } else {
                // Scan alphanumeric + _ + ?
                while let Some(&vc) = chars.peek() {
                    if vc.is_alphanumeric() || vc == '_' || vc == '?' {
                         var_name.push(chars.next().unwrap());
                         // $? is special, single char
                         if var_name == "?" { break; }
                    } else {
                        break;
                    }
                }
            }

            if var_name.is_empty() {
                // Just a $
                current_token.push('$');
            } else if var_name == "?" {
                current_token.push_str(&ctx.exit_code.to_string());
            } else {
                if let Some(val) = ctx.env.get(&var_name) {
                    current_token.push_str(val);
                }
            }
            continue;
        }

        if c.is_whitespace() {
            if in_single_quote || in_double_quote {
                current_token.push(c);
                token_started = true;
            } else if token_started {
                args.push(normalize_path(current_token));
                current_token = String::new();
                token_started = false;
            }
            continue;
        }

        current_token.push(c);
        token_started = true;
    }

    if token_started {
        args.push(normalize_path(current_token));
    } else if escaped {
        // Trailing backslash? e.g. "echo \"
        // Should push backslash? Or error?
        // Shells usually wait for more input. Here we just push empty or backslash?
        // Logic above: `escaped = true`. Loop ends.
        // We probably want to push nothing or warn.
        // If I typed `echo \`, I expect `echo`? No, `\` escapes newline usually.
        // For Mini-Parser, let's ignore or push empty.
    }

    Ok(args)
}

fn normalize_path(token: String) -> String {
    if cfg!(windows) {
        if token.contains('/') {
            return token.replace('/', &MAIN_SEPARATOR.to_string());
        }
    }
    // On Unix, do not normalize \ to / as \ is valid filename char.
    token
}
