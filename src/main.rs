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
            Some(_) => {
                end += 1;
            }
            None => break,
        }
    }

    let length = end.saturating_sub(start);
    Ok((JsonTokens::String(&content[start..end], length), length + 2))
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
                let (token, length) = scan_string(&content[current + 1..], 0)?;
                tokens.push(token);
                current += length;
                continue;
            }
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => {
                //FIXME: There is no identifier JSON only strings, bools, numbers and nulls
                //so this should only check if we have true,false values or null
                let (token, length) = scan_identifier(&content, current)?;
                tokens.push(token);
                //TODO: Refactor this into a Lexer struct which keeps the internal
                //state globally available for all consumers so we don't need to
                //take the consumed length from a level above us into account.
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

//TODO: These should take more information i.e. where did the error occured,
//      what exactly did ruffle the parses feathers the wrong way ?
//TODO: Name these Error Types better
#[derive(Debug, Clone)]
enum ParserError {
    InvalidValueInArray,
    InvalidSymbolInCurrentContext,
    InvalidKey,
    MissingSymbol,
    InvalidValueInCurrentContext,
    EmptyJson,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValueInArray => write!(f, "todo"),
            Self::InvalidSymbolInCurrentContext => write!(f, "todo"),
            Self::InvalidKey => write!(f, "todo"),
            Self::MissingSymbol => write!(f, "todo"),
            Self::InvalidValueInCurrentContext => write!(f, "todo"),
            Self::EmptyJson => write!(f, "todo"),
        }
    }
}

impl std::error::Error for ParserError {
    fn description(&self) -> &str {
        "JSON Parser Error"
    }
}

struct Parser<'a> {
    position: usize,
    tokens: &'a [JsonTokens<'a>],
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [JsonTokens]) -> Self {
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

    fn parse_json_array(&mut self) -> Result<()> {
        dbg!(&self.tokens[self.position]);

        loop {
            //OPENBRACE STRING|NUMBER|BOOLEAN COMMA STRING|NUMBER|BOOLEAN ... CLOSINGBRACE
            match self.advance() {
                Some(JsonTokens::String(value, _)) => println!("Element: {}", value),
                Some(JsonTokens::OpenCurlyBrace) => self.parse_json_object()?,
                _ => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            };

            match self.advance() {
                Some(JsonTokens::ClosingSquareBrace) => break,
                Some(JsonTokens::Comma) => self.advance(),
                _ => todo!(),
            };
        }

        Ok(())
    }

    fn parse_key_value_pair(&mut self) -> Result<()> {
        dbg!(&self.tokens[self.position]);
        loop {
            match self.advance() {
                Some(JsonTokens::Identifier(key, _)) => println!("{}", key),
                Some(JsonTokens::String(key, _)) => println!("{}", key),
                _ => return Err(ParserError::InvalidKey.into()),
            };

            if !self.match_token(JsonTokens::Colon) {
                return Err(ParserError::InvalidSymbolInCurrentContext.into());
            }

            self.advance();

            match self.advance() {
                Some(JsonTokens::Identifier(value, _)) => println!("{}", value),
                Some(JsonTokens::String(value, _)) => println!("{}", value),
                Some(JsonTokens::OpenCurlyBrace) => self.parse_json_object()?,
                Some(JsonTokens::OpenSquareBrace) => self.parse_json_array()?,
                _ => return Err(ParserError::InvalidValueInCurrentContext.into()), 
            };

            if !self.match_token(JsonTokens::Comma) {
                break;
            }

            self.advance();
        }

        Ok(())
    }

    fn parse_json_object(&mut self) -> Result<()> {
        //FIXME: Potential to blow the stack if we recurse to deep !
        //TODO: Collapse these Error, where possible into one branch
        match self.peek() {
            Some(JsonTokens::OpenCurlyBrace) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::ClosingCurlyBrace) => return Ok(()),
            Some(JsonTokens::OpenSquareBrace) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::ClosingSquareBrace) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::Colon) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::Comma) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::Identifier(_, _)) => self.parse_key_value_pair()?,
            Some(JsonTokens::String(_, _)) => self.parse_key_value_pair()?,
            Some(JsonTokens::Eof) => return Err(ParserError::EmptyJson.into()),
            None => return Err(ParserError::EmptyJson.into()),
        };

        if !self.match_token(JsonTokens::ClosingCurlyBrace) {
            return Err(ParserError::MissingSymbol.into());
        }

        Ok(())
    }

    fn parse(&mut self) -> Result<()> {
        match self.advance() {
            Some(JsonTokens::OpenCurlyBrace) => self.parse_json_object()?,
            Some(JsonTokens::ClosingCurlyBrace) => return Err(ParserError::InvalidSymbolInCurrentContext.into()), 
            Some(JsonTokens::OpenSquareBrace) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::ClosingSquareBrace) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::Colon) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::Comma) => return Err(ParserError::InvalidSymbolInCurrentContext.into()),
            Some(JsonTokens::Identifier(_, _)) => return Err(ParserError::InvalidValueInCurrentContext.into()),
            Some(JsonTokens::String(_, _)) => return Err(ParserError::InvalidValueInCurrentContext.into()),
            Some(JsonTokens::Eof) => return Err(ParserError::InvalidValueInCurrentContext.into()),
            None => todo!(),
        };

        Ok(())
    }
}

fn main() -> Result<()> {
    let file_content = fs::read_to_string("examples/test.json")?;

    let mut stdout = io::stdout();
    let _ = stdout.lock();

    //TODO: For now
    let lexed_json = scan_json(&file_content)?;
    //parse_json(&lexed_json);

    let mut parser = Parser::new(&lexed_json);
    parser.parse()?;

    if lexed_json.first() == Some(&JsonTokens::Eof) {
        let mut stderr = io::stderr();
        let _ = stderr.lock();

        write!(stderr, "Invalid Json file")?;
        process::exit(1);
    }

    write!(stdout, "{}", &file_content)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::*;
    #[test]
    fn test_json_empty_is_invalid() {
        //A empty file is invalid json by the definition of the standard
        //we need __at least__ a empty json object
        let test_src = "";

        let tokens = scan_json(&test_src).unwrap();
        let parser = Parser::new(&tokens).parse();

        assert_eq!(parser.is_err(), true);
    }

    #[test]
    fn test_json_empty_object_is_valid() {
        //A file with a empty json object is valid json
        let test_src = "{}";

        let tokens = scan_json(&test_src).unwrap();
        let parser = Parser::new(&tokens).parse();

        assert_eq!(parser.is_ok(), true);
    }

    #[test]
    fn test_json_key_without_a_value_is_invalid() {
        //A json rootobject with a key that has no valid value is invalid json
        let test_src = "{\"some_key\":}";

        let tokens = scan_json(&test_src).unwrap();
        let parser = Parser::new(&tokens).parse();

        assert_eq!(parser.is_err(), true);
    }

    #[test]
    fn test_json_key_with_value_but_no_closing_brace_is_invalid() {
        //A json rootobject with a key and a string value but a missing closing
        //curly brace is invalid json
        let test_src = "{ \"some_key\": \"some_value\" ";

        let tokens = scan_json(&test_src).unwrap();
        let parser = Parser::new(&tokens).parse();

        assert_eq!(parser.is_err(), true);
    }

    #[test]
    fn test_json_key_with_string_value_is_valid() {
        //A json rootobject with a key and a string value is valid json
        let test_src = "{ \"some_key\": \"some_value\" }";

        let tokens = scan_json(&test_src).unwrap();
        let parser = Parser::new(&tokens).parse();

        assert_eq!(parser.is_ok(), true);
    }

    #[test]
    fn test_json_key_with_value_and_trailing_comma_is_invalid() {
        //A json rootobject with a kv pair is invalid with a trailing comma
        let test_src = "{ \"some_key\": \"some_value\", }";

        let tokens = scan_json(&test_src).unwrap();
        let parser = Parser::new(&tokens).parse();

        assert_eq!(parser.is_err(), true);
    }

    #[test]
    fn test_json_key_with_value_comma_key_with_value_is_valid() {
        //A kv pair after a comma results in a valid json
        let test_src = "{ \"some_key\": \"some_value\", \"s2\": \"v2\" }";

        let tokens = scan_json(&test_src).unwrap();
        let parser = Parser::new(&tokens).parse();

        assert_eq!(parser.is_ok(), true);
    }
}
