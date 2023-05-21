use anyhow::Result;
use std::fs; 
use std::io;
use std::io::Write;
use std::process;


#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum JsonTokens<'a> {
    OpenCurlyBrace,
    ClosingCurlyBrace,
    OpenSquareBrace,
    ClosingSquareBrace,
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
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => {
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
            b'[' => tokens.push(JsonTokens::OpenSquareBrace),
            b']' => tokens.push(JsonTokens::ClosingSquareBrace),
            b':' => tokens.push(JsonTokens::Colon),
            b',' => tokens.push(JsonTokens::Comma),
            b'"' => {
                let (token, length) = scan_string(&content[current+1..], 0)?;
                tokens.push(token);
                current += length;
                continue
            },
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => {
                let (token, length) = scan_identifier(&content, current)?;
                tokens.push(token);
                current += length;
                continue;
            }

            unknown => panic!("The lexer hit a unknown symbol please add {}", unknown),
        }

        current += 1;
    }

    tokens.push(JsonTokens::Eof);
    dbg!(&tokens);
    Ok(tokens)
}


struct Parser<'a> {
    position: usize,
    tokens: &'a [JsonTokens<'a>],
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a[JsonTokens]) -> Self {
        Self {
            position: 0,
            tokens,
        }
    }

    fn match_token(&mut self, to_match: JsonTokens) -> bool {
        if self.tokens[self.position] == to_match {
            return true;
        }

        false
    }

    fn peek(&mut self) -> Option<&JsonTokens> {
        Some(&self.tokens[self.position])
    }

    fn advance(&mut self) -> Option<&JsonTokens> {
        let next_token = &self.tokens[self.position];
        self.position += 1;

        Some(next_token)
    }

    fn parse_json_array(&mut self) {
        dbg!(&self.tokens[self.position]);
        loop {

            if self.match_token(JsonTokens::ClosingSquareBrace) {
                self.advance();
                break;
            }

            match self.advance() {
                Some(JsonTokens::String(value, _)) => println!("Element: {}", value),
                _ => todo!(),
            }

            if !self.match_token(JsonTokens::Comma) {
                break;
            }

            self.advance();
        }
    }

    fn parse_key_value_pair(&mut self) {
        dbg!(&self.tokens[self.position]);
        loop {
            match self.advance() {
                Some(JsonTokens::Identifier(key, _)) => println!("{}", key),
                Some(JsonTokens::String(key, _)) => println!("{}", key),
                _ => crash_and_burn("key needs to be a string or a identifier"),
            }

            if !self.match_token(JsonTokens::Colon) {
                crash_and_burn("expected colon after key");
                break;
            }

            self.advance();

            match self.advance() {
                Some(JsonTokens::Identifier(value, _)) => println!("{}", value),
                Some(JsonTokens::String(value, _)) => println!("{}", value),
                Some(JsonTokens::OpenCurlyBrace) => self.parse_json_object(),
                Some(JsonTokens::OpenSquareBrace) => self.parse_json_array(),
                _ => crash_and_burn("value needs to be a string or a identifier"),
            }

            if !self.match_token(JsonTokens::Comma) {
                break;
            }

            self.advance();
        }
    }

    fn parse_json_object(&mut self) {
        //FIXME: Potential to blow the stack if we recurse to deep !
        match self.peek() {
            Some(JsonTokens::OpenCurlyBrace) => self.parse_json_object(),
            Some(JsonTokens::ClosingCurlyBrace) => return,
            Some(JsonTokens::OpenSquareBrace) => todo!(),
            Some(JsonTokens::ClosingSquareBrace) => todo!(),
            Some(JsonTokens::Colon) => todo!(),
            Some(JsonTokens::Comma) => todo!(),
            Some(JsonTokens::Identifier(_, _)) => self.parse_key_value_pair(),
            Some(JsonTokens::String(_, _)) => self.parse_key_value_pair(),
            Some(JsonTokens::Eof) => crash_and_burn("Unclosed JSON Object"),
            None => crash_and_burn("Expected '}' but tokenstream was empty"),
        }
    }

    fn parse(&mut self) {
        match self.advance() {
            Some(JsonTokens::OpenCurlyBrace) => self.parse_json_object(),
            Some(JsonTokens::ClosingCurlyBrace) => crash_and_burn("expected the start of a json object"),
            Some(JsonTokens::OpenSquareBrace) => crash_and_burn("expected the start of a json object but got open square brace"),
            Some(JsonTokens::ClosingSquareBrace) => crash_and_burn("expected the start of a json object but got closed square brace"),
            Some(JsonTokens::Colon) => crash_and_burn("expected the start of a json object but found a colon"),
            Some(JsonTokens::Comma) => crash_and_burn("expected the start of a json object but found a comma"),
            Some(JsonTokens::Identifier(_, _)) => crash_and_burn("identifer outside json object"),
            Some(JsonTokens::String(_, _)) => crash_and_burn("string outside of json object"),
            Some(JsonTokens::Eof) => crash_and_burn("Empty JSON"),
            None => todo!(),
        }
    }
}

//TODO: This thingy should be replace with a Error enum and a result type on
//      the parser methods.
fn crash_and_burn(message: &str) {
    let mut stderr = io::stderr();
    let _ = stderr.lock();

    write!(stderr, "ERROR: {}", message).unwrap();
    process::exit(1);
}
fn main() -> Result<()> {
    let file_content = fs::read_to_string("examples/test.json")?;

    let mut stdout = io::stdout();
    let _ = stdout.lock();

    //TODO: For now 
    let lexed_json = scan_json(&file_content)?;
    //parse_json(&lexed_json);

    let mut parser = Parser::new(&lexed_json);
    parser.parse();

    if lexed_json.first() == Some(&JsonTokens::Eof) {
        let mut stderr = io::stderr();
        let _ = stderr.lock();

        write!(stderr, "Invalid Json file")?;
        process::exit(1);
    }

    write!(stdout, "{}", &file_content)?;

    Ok(())
}
