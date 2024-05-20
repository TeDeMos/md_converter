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
    #![allow(clippy::too_many_lines)]
    #[must_use] pub fn parse_lines(lines: &[String]) -> Vec<Inline> {
        //todo
        let mut result = Vec::new();
        let ascii_punctuation = [
            '!', '"', '#', '%', '&', '\'', '(', ')', '*', ',', '.', '/', ':', ';', '?', '@', '[',
            '\\', ']', '^', '_', '`', '{', '}', '|', '~',
        ];
        let mut space = false;
        let mut current = String::new();
        let mut html_entity_state = HtmlEntityState::NoState;
        for l in lines {
            let mut char_iter = l.trim().char_indices().peekable();
            let mut start_slice_index: usize = 0;
            let mut length: u32 = 0;
            loop {
                if matches!(html_entity_state, HtmlEntityState::Dec | HtmlEntityState::Hex){
                    match char_iter.next() {
                        Some((end_index,';')) => {
                            length = 0;
                            let entity_value = u32::from_str_radix(&l.trim()[start_slice_index..end_index],html_entity_state.get_base());
                            match entity_value {
                                Ok(value) => {current.push(from_u32(value).unwrap())},
                                Err(_) => {current.push(from_u32(0xFFFD).unwrap())}
                            }
                            html_entity_state = HtmlEntityState::NoState;
                        },
                        Some((_,_c@ ('0'..='9'))) if length < html_entity_state.get_entity_max_length() => {
                                length += 1;
                        }
                        Some((_,_c @ ( 'a'..='f' | 'A'..='F'))) if html_entity_state.get_base() == 16 => {
                            if length < html_entity_state.get_entity_max_length() {
                                length += 1;
                            }
                        }
                        //This could be written better I think
                        Some((_,_c @ ('x' | 'X'))) if html_entity_state.get_base() == 16 => {
                            continue;
                        }
                        
                        Some((end_index,_)) => {
                            html_entity_state = HtmlEntityState::NoState;
                            length = 0;
                            current.push_str(&l.trim()[start_slice_index..end_index]);
                        },
                        None => {}
                    }
                    continue;
                }
                match char_iter.next() {
                    Some((_,' ' |'\t')) if !space => {
                        result.push(Inline::Str(current));
                        current = String::new();
                        result.push(Inline::Space);
                        space = true;
                    },
                    Some((_,' ' | '\t')) if space => {},
                    Some((_,'\\')) => if let Some((_,c)) = char_iter.next() {
                        if !ascii_punctuation.contains(&c) {
                            current.push('\\');
                        }
                        current.push(c);
                    } else {
                        if !current.is_empty() {
                            result.push(Inline::Str(current));
                            current = String::new();
                        }
                        result.push(Inline::LineBreak);
                        break;
                    },
                    Some((pos,'&')) => {
                        start_slice_index = pos;
                        match char_iter.peek() {
                            Some((_,'#')) => {
                                char_iter.next();
                                match char_iter.peek() {
                                    Some((index, _c @ ('X' | 'x'))) => {
                                        html_entity_state = HtmlEntityState::Hex;
                                        start_slice_index = *index+1;
                                    }
                                    Some((index,_)) => {
                                        html_entity_state = HtmlEntityState::Dec;
                                        start_slice_index = *index;
                                    }
                                    None => {}
                                }
                            },
                            Some(_c) => {todo!()},
                            None => {},
                        }
                    },
                    Some((_,c)) => {
                        current.push(c);
                    }
                    None => {
                        if !current.is_empty() {
                            result.push(Inline::Str(current));
                            current = String::new();
                        }
                        result.push(Inline::SoftBreak);
                        break;
                    },
                }
            }
        }
        result.pop();
        result
    }



    pub fn parse_line(line: String) -> Vec<Inline> { Self::parse_lines(slice::from_ref(&line)) }

    fn _parse_one_line(line: &str, result: &mut Vec<Inline>) {
        //todo
        let mut space = false;
        let mut current = String::new();
        for c in line.trim().chars() {
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
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use crate::inline_parser::*;
    #[test]
    fn test_test() {
        // let result = MdReader::read("> ```\n> aaa\n\nbbb").into_ok();
        let test = vec!["hello        rust \\' \\ab".to_string()];
        let result = InlineParser::parse_lines(&test);
        let Inline::Str(s) = &result[2] else { return };
        for c in s.chars() {
            println!("{}", c);
        }
        dbg!(result);
    }
    
    #[test]
    fn html_entity_dec_test() {
        let test = vec!["&#42;  asdfsasdasdasffs".to_string()];
        let result = InlineParser::parse_lines(&test);
        let Inline::Str(s) = &result[0] else {return};
        assert_eq!(s.to_string(),String::from("*"));
        assert_eq!(Inline::Space,result[1]);
        let Inline::Str(s) = &result[2] else {return};
        assert_eq!(s.to_string(),String::from("asdfsasdasdasffs"));
    }
    
    #[test]
    fn html_entity_hex_test() {
        use crate::inline_parser::*;
        let test = vec!["&#x2A;  asdfsasdasdasffsasdf".to_string()];
        let result = InlineParser::parse_lines(&test);
        let Inline::Str(s) = &result[0] else {return};
        assert_eq!(s.to_string(),String::from("*"));
        assert_eq!(Inline::Space,result[1]);
        let Inline::Str(s) = &result[2] else {return};
        assert_eq!(s.to_string(),String::from("asdfsasdasdasffs"));
    }
}
