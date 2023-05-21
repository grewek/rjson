use anyhow::Result;
use std::fs; 
use std::io;
use std::io::Write;
use std::process;


#[derive(Debug, PartialEq, Eq)]
enum JsonTokens<'a> {
    OpenCurlyBrace,
    ClosingCurlyBrace,
    Colon,
    Comma,
    Identifier(&'a str, usize),
    String(&'a str, usize),
    Eof,
}

fn scan_string<'a>(content: &'a str, start: usize) -> Result<(JsonTokens, usize)> {
    let mut end = start + 1;
    let bytes = &mut content.as_bytes();
    while end < bytes.len() {
        match bytes.get(end) {
            Some(b'"') => break, 
            Some(_) =>{
                end += 1;
            }
            None => break,
        }
    }

    let length = end.saturating_sub(start);
    Ok((JsonTokens::String(&content[start..end], length), length+2))
}

fn scan_identifier<'a>(content: &'a str, start: usize) -> Result<(JsonTokens, usize)> {
    let mut end = start;
    let bytes = &mut content.as_bytes();
    while end < bytes.len() {
        match bytes[end] {
            b'a'..=b'z' | b'A'..=b'Z' => {
                end += 1;
            }
            _ => break,
        }
    }

    let length = end.saturating_sub(start);
    Ok((JsonTokens::Identifier(&content[start..end], length), length))
}

fn scan_json(content: &str) -> Result<Vec<JsonTokens>> {
    let mut tokens = vec![];
    let bytes = &mut content.as_bytes();
    let mut current = 0;

    while current < bytes.len() {
        if bytes[current].is_ascii_whitespace() {
            current += 1;
            continue;
        }

        match bytes[current] {
            b'{' => tokens.push(JsonTokens::OpenCurlyBrace),
            b'}' => tokens.push(JsonTokens::ClosingCurlyBrace),
            b':' => tokens.push(JsonTokens::Colon),
            b',' => tokens.push(JsonTokens::Comma),
            b'"' => {
                let (token, length) = scan_string(&content[current+1..], 0)?;
                tokens.push(token);
                current += length;
                continue
            },
            b'a'..=b'z' | b'A'..=b'Z' => {
                let (token, length) = scan_identifier(&content, current)?;
                tokens.push(token);
                current += length;
                continue;
            }

            _ => panic!("whoopsie: {:?}", bytes[current] as char),
        }

        current += 1;
    }

    tokens.push(JsonTokens::Eof);
    dbg!(&tokens);
    Ok(tokens)
}


fn parse_key_value_pair(tokens: &[JsonTokens]) {
    let mut current = 0;
    loop {
        let key = &tokens.get(current);
        let colon = &tokens.get(current+1);
        let value = &tokens.get(current+2);
        match (key, colon, value) {
            (Some(JsonTokens::Identifier(key, _)), Some(JsonTokens::Colon), Some(JsonTokens::Identifier(value, _))) => 
                println!("{} {}", key, value),
            (Some(JsonTokens::String(key, _)), Some(JsonTokens::Colon), Some(JsonTokens::String(value, _))) => 
                println!("{} {}", key, value),
            (Some(JsonTokens::Identifier(_, _)), Some(_), Some(_)) => 
                crash_and_burn("colon expected after key value"),
            _ => 
                crash_and_burn(format!("Another error {:?} {:?} {:?}", key, colon, value).as_str()),
        }

        match &tokens.get(current+3) {
            Some(JsonTokens::Comma) => current += 4,
            _ => break,
        }
    }
}
fn parse_json_object(tokens: &[JsonTokens]) {
    match tokens.first() {
        Some(JsonTokens::OpenCurlyBrace) => parse_json_object(&tokens[1..]),
        Some(JsonTokens::Colon) => todo!(),
        Some(JsonTokens::Comma) => todo!(),
        Some(JsonTokens::ClosingCurlyBrace) => return,
        Some(JsonTokens::Identifier(_, _)) => parse_key_value_pair(&tokens),
        Some(JsonTokens::String(_, _)) => parse_key_value_pair(&tokens),
        Some(JsonTokens::Eof) => crash_and_burn("Unclosed JSON Object"),
        None => crash_and_burn("Expected '}' but tokenstream was empty"),
    }
}
fn parse_json(tokens: &[JsonTokens]) {
    match tokens.first() {
        Some(JsonTokens::OpenCurlyBrace) => parse_json_object(&tokens[1..]),
        Some(JsonTokens::Colon) => todo!(),
        Some(JsonTokens::Comma) => todo!(),
        Some(JsonTokens::ClosingCurlyBrace) => crash_and_burn("Expected the start of a json object"),
        Some(JsonTokens::Identifier(_, _)) => crash_and_burn("identifer outside json object"),
        Some(JsonTokens::String(_, _)) => crash_and_burn("string outside of json object"),
        Some(JsonTokens::Eof) => crash_and_burn("Empty JSON"),
        None => todo!(),
    }
}

fn crash_and_burn(message: &str) {
    let mut stderr = io::stderr();
    let _ = stderr.lock();

    write!(stderr, "ERROR: {}", message).unwrap();
    process::exit(1);
}
fn main() -> Result<()> {
    let file_content = fs::read_to_string("examples/step1/test7.json")?;

    let mut stdout = io::stdout();
    let _ = stdout.lock();

    let lexed_json = scan_json(&file_content)?;
    parse_json(&lexed_json);

    if lexed_json.first() == Some(&JsonTokens::Eof) {
        let mut stderr = io::stderr();
        let _ = stderr.lock();

        write!(stderr, "Invalid Json file")?;
        process::exit(1);
    }

    write!(stdout, "{}", &file_content)?;

    Ok(())
}
