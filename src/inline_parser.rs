use std::char::from_u32;
use std::slice;
use crate::ast::Inline;

pub struct InlineParser;

enum HtmlEntityState {
    Hex,
    Dec,
    NoState,
}

impl HtmlEntityState {
    const fn get_entity_max_length(&self) -> u32 {
        match self {
            Self::Hex        =>  5,
            Self::Dec        =>  6,
            Self::NoState    =>  0,
        }
    }

    const fn get_base(&self) -> u32 {
        match self {
            Self::Hex        =>  16,
            Self::Dec        =>  10,
            Self::NoState    =>  0,
        }
    }
}

impl InlineParser {
    pub fn parse_lines(lines: &str) -> Vec<Inline> {
        let mut result = Vec::new();
        let mut space = false;
        let mut current = String::new();
        for c in lines.trim().chars() {
            if space {
                if !matches!(c, ' ' | '\t') {
                    result.push(Inline::Space);
                    space = false;
                    current.push(c);
                }
            } else if matches!(c, ' ' | '\t') {
                result.push(Inline::Str(current));
                current = String::new();
                space = true;
            } else {
                current.push(c);
            }
        }
        if !current.is_empty() {
            result.push(Inline::Str(current));
        }
        result
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use crate::inline_parser::*;
    #[test]
    fn test_test() {
        // let result = MdReader::read("> ```\n> aaa\n\nbbb").into_ok();
        let test = "hello        rust \\' \\ab".to_string();
        let result = InlineParser::parse_lines(&test);
        assert_eq!(Inline::Str("hello".to_string()), result[0]);
        assert_eq!(Inline::Space, result[1]);
        assert_eq!(Inline::Str("rust".to_string()), result[2]);
        assert_eq!(Inline::Space, result[3]);
        assert_eq!(Inline::Str("'".to_string()), result[4]);
        assert_eq!(Inline::Space, result[5]);
    }
    
    #[test]
    fn html_entity_dec_test() {
        let test = "&#42;  asdfsasdasdasffs".to_string();
        let result = InlineParser::parse_lines(&test);
        let Inline::Str(s) = &result[0] else {return};
        assert_eq!(s.to_string(),String::from("*"));
        assert_eq!(Inline::Space,result[1]);
        let Inline::Str(s) = &result[2] else {return};
        assert_eq!(s.to_string(),String::from("asdfsasdasdasffs"));
    }
    
    #[test]
    fn html_entity_hex_test() {
        let test = "&#x2A;  asdfsasdasdasffsasdf".to_string();
        let result = InlineParser::parse_lines(&test);
        let Inline::Str(s) = &result[0] else {return};
        assert_eq!(s.to_string(),String::from("*"));
        assert_eq!(Inline::Space,result[1]);
        let Inline::Str(s) = &result[2] else {return};
        assert_eq!(s.to_string(),String::from("asdfsasdasdasffsasdf"));
    }
}
