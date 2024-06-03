use std::collections::HashMap;
use std::fs;
use std::iter::Peekable;
use std::str::CharIndices;
use std::string::String;
use lazy_static::lazy_static;

use serde::Deserialize;

use crate::ast::{attr_empty, Inline};

pub struct InlineParser {}

enum HtmlEntityState {
    Hex,
    Dec,
    NoState,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Potential {
    Opener,
    Closer,
    Both,
    None,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct DelimiterStruct<'a> {
    count: usize,
    is_strong: bool,
    delimiter_char: char,
    delim_slice: &'a str,
    typeof_delimiter: Potential,
    temp_vec: Vec<usize>,
}

#[allow(dead_code)]
impl<'a> DelimiterStruct<'a> {
    fn change_slice(&mut self, new_slice: &'a str) { self.delim_slice = new_slice; }
}

#[allow(dead_code)]
#[derive(Clone)]
struct InlineElement<'a> {
    slice: &'a str,
    element: Inline,
}

impl HtmlEntityState {
    const fn get_entity_max_length(&self) -> u32 {
        match self {
            Self::Hex => 5,
            Self::Dec => 6,
            Self::NoState => 0,
        }
    }

    const fn get_base(&self) -> u32 {
        match self {
            Self::Hex => 16,
            Self::Dec => 10,
            Self::NoState => 0,
        }
    }
}

struct BacktickString {
    backtick_length: usize,
    start_index: usize,
}

enum SliceVariant<'a> {
    CodeSlice(&'a str),
    InlineSlice(&'a str),
}
#[allow(dead_code)]
enum StringOrChar {
    NoHTMLString(String),
    HTMLChar(char),
}

lazy_static! {
    static ref ENTITIES: HashMap<String, String> = {
        let vec: Vec<(String, String)> = serde_json::from_str(&fs::read_to_string("entities.json").unwrap()).unwrap();
        vec.into_iter().collect()
    };
}

impl InlineParser {
    fn get_backtick_string_length_vector(paragraph: &str) -> Vec<BacktickString> {
        let mut iter = paragraph.char_indices();
        let mut result = Vec::new();
        loop {
            match iter.next() {
                Some((s, '`')) => loop {
                    match iter.next() {
                        Some((_, '`')) => continue,
                        Some((e, _)) => {
                            result.push(BacktickString { backtick_length: e - s, start_index: s });
                            break;
                        },
                        None => {
                            result.push(BacktickString {
                                backtick_length: paragraph.len() - s,
                                start_index: s,
                            });
                            break;
                        },
                    }
                },
                Some(_) => continue,
                None => return result,
            }
        }
    }

    #[allow(dead_code)]
    fn contains_slice(outer: &str, inner: &str) -> bool {
        let outer_start = outer.as_ptr() as usize;
        let outer_end = outer_start + outer.len();
        let inner_start = inner.as_ptr() as usize;
        let inner_end = inner_start + inner.len();

        inner_start >= outer_start && inner_end <= outer_end
    }

    fn parse_backtick_string_length_vector(
        paragraph: &str, backtick_vec: Vec<BacktickString>,
    ) -> Vec<SliceVariant> {
        let mut open_iter = backtick_vec.iter();
        let mut prev_index = 0;
        let mut result: Vec<SliceVariant> = Vec::new();
        loop {
            if let Some(c) = open_iter.next() {
                if (c.start_index != 0) {
                    result.push(SliceVariant::InlineSlice(&paragraph[prev_index..c.start_index]));
                    prev_index = c.start_index;
                }
                let current = c;
                let mut close_iter = open_iter.clone();
                while let Some(c) = close_iter.next() {
                    if current.backtick_length == c.backtick_length {
                        result.push(SliceVariant::CodeSlice(
                            &paragraph[current.start_index..c.start_index + c.backtick_length],
                        ));
                        prev_index = c.start_index + c.backtick_length;
                        open_iter = close_iter;
                        break;
                    }
                }
            } else {
                if (prev_index != paragraph.len()) {
                    result.push(SliceVariant::InlineSlice(&paragraph[prev_index..paragraph.len()]));
                }
                return result;
            }
        }
    }

    fn parse_code_spans(paragraph: &str) -> Vec<SliceVariant> {
        let mut backticks: Vec<BacktickString> = Self::get_backtick_string_length_vector(paragraph);
        let mut result = Self::parse_backtick_string_length_vector(paragraph, backticks);
        return result;
    }

    pub fn html_entity_parsing(char_iter: &Peekable<CharIndices>) {}

    pub fn parse_html_entities(paragraph: &str) -> String {
        let mut chars = paragraph.chars();
        let mut new_paragraph = String::new();
        let mut current = String::new();
        loop {
            match chars.next() {
                Some('&') => {
                    let mut temp_iter = chars.clone();
                    current = String::from("&");
                    loop {
                        match temp_iter.next() {
                            Some(';') => {
                                match ENTITIES.get(&current) {
                                    Some(c) => {
                                        new_paragraph.push_str(c);
                                    },
                                    None => {
                                        new_paragraph.push('&');
                                    },
                                }
                                chars = temp_iter.clone();
                                break;
                            },
                            Some(x) => {
                                current.push(x);
                            },
                            None => {
                                new_paragraph.push_str(&*current);
                                return new_paragraph;
                            },
                        }
                    }
                },
                Some(c) => {
                    new_paragraph.push(c);
                },
                None => {
                    break;
                },
            }
        }
        new_paragraph
    }

    pub fn parse_lines(paragraph: &str) -> Vec<Inline> {
        let mut new_paragraph = &Self::parse_html_entities(paragraph);
        let inlines_and_code = Self::parse_code_spans(new_paragraph);
        let mut last_opener_star: [Option<usize>; 3] = [None; 3];
        let mut last_opener_floor: [Option<usize>; 3] = [None; 3];
        let mut result: Vec<InlineElement> = Vec::new();
        let mut delimiter_stack: Vec<DelimiterStruct> = Vec::new();
        let mut iter = inlines_and_code.iter();
        let mut is_beginning: bool = true;

        loop {
            match iter.next() {
                Some(&SliceVariant::CodeSlice(x)) => {
                    // Check if emphasis open then prepare the CODE inline
                    result.push(Self::parse_code_slice(x));
                    // println!("Code {x}");
                    is_beginning = false;
                },
                Some(&SliceVariant::InlineSlice(x)) => {
                    delimiter_stack.append(&mut Self::parse_inline_slice(
                        x, &mut result, &mut last_opener_star, &mut last_opener_floor, is_beginning,
                    ));
                    is_beginning = false;
                    // println!("Inline {x}");
                },
                None => break,
            }
        }
        let mut true_result: Vec<Inline> = vec![];
        let mut is_prev_str = false;

        let mut in_vec = parse_emph(
            new_paragraph, &mut delimiter_stack, &last_opener_star, &last_opener_floor, 0,
            &mut result,
        );

        for x in &result {
            match x.element.clone() {
                Inline::Str(c) | Inline::Temp(c) =>
                    if is_prev_str {
                        let mut temp = true_result.pop().unwrap();
                        if let Inline::Str(y) = temp {
                            true_result.push(Inline::Str(y.to_string() + &*c.to_string()));
                        }
                    } else {
                        true_result.push(Inline::Str(c.to_string()));
                        is_prev_str = true;
                    },
                Inline::None => {},
                c => {
                    true_result.push(c);
                    is_prev_str = false;
                },
            }
            // true_result.push(x.element);
            // println!("{:?}", x.element);
        }
        for x in &true_result {
            if *x != Inline::None {
                print!("{:?} ", x);
            }
        }
        return true_result;
    }

    fn parse_code_slice(slice: &str) -> InlineElement {
        let result = slice[1..slice.len() - 1].replace('\n', " ");
        if !result.chars().all(|c| matches!(c, ' '))
            && result.starts_with(' ')
            && result.ends_with(' ')
        {
            InlineElement {
                element: Inline::Code(attr_empty(), result[1..result.len() - 1].parse().unwrap()),
                slice,
            }
        } else {
            InlineElement { element: Inline::Code(attr_empty(), result.parse().unwrap()), slice }
        }
    }

    fn parse_hex_entity(
        mut copy_iter: Peekable<CharIndices>,
    ) -> (StringOrChar, Peekable<CharIndices>) {
        let begin_iter = copy_iter.clone();
        let mut current_bonus = String::new();
        let mut length = 0;
        loop {
            if length > 6 {
                break;
            };

            match copy_iter.peek() {
                Some((_, c @ ('0'..='9' | 'a'..='f' | 'A'..='F'))) => {
                    current_bonus.push(*c);
                    copy_iter.next();
                },
                Some((_, c @ ';')) =>
                    if !current_bonus.is_empty() {
                        let entity_value = u32::from_str_radix(&current_bonus, 16);
                        return match entity_value {
                            Ok(x) => {
                                copy_iter.next();
                                (StringOrChar::HTMLChar(char::from_u32(x).unwrap()), copy_iter)
                            },
                            Err(_) =>
                                (StringOrChar::HTMLChar(char::from_u32(0xfffd).unwrap()), copy_iter),
                        };
                    },
                Some((_, c)) => return (StringOrChar::NoHTMLString(current_bonus), begin_iter),
                None => {},
            }
            length += 1;
        }
        return (StringOrChar::NoHTMLString(current_bonus), begin_iter);
    }

    fn parse_dec_entity(
        mut copy_iter: Peekable<CharIndices>,
    ) -> (StringOrChar, Peekable<CharIndices>) {
        let begin_iter = copy_iter.clone();
        let mut current_bonus = String::new();
        let mut length = 0;
        loop {
            if length > 7 {
                break;
            };
            match copy_iter.peek() {
                Some((_, c @ ('0'..='9'))) => {
                    current_bonus.push(*c);
                    copy_iter.next();
                },
                Some((_, c @ ';')) =>
                    if !current_bonus.is_empty() {
                        let entity_value = u32::from_str_radix(&*current_bonus, 10);
                        return match entity_value {
                            Ok(x) => {
                                copy_iter.next();
                                (StringOrChar::HTMLChar(char::from_u32(x).unwrap()), copy_iter)
                            },
                            Err(_) =>
                                (StringOrChar::HTMLChar(char::from_u32(0xfffd).unwrap()), copy_iter),
                        };
                    },
                Some((_, c)) => return (StringOrChar::NoHTMLString(current_bonus), begin_iter),
                None => return (StringOrChar::NoHTMLString(current_bonus), begin_iter),
            }
            length += 1;
        }
        return (StringOrChar::NoHTMLString(current_bonus), begin_iter);
    }

    fn change_to_base(slice1: &str, slice2: &str) -> usize {
        return slice1.as_ptr() as usize - slice2.as_ptr() as usize;
    }

    #[allow(clippy::too_many_lines)]
    fn parse_inline_slice<'a>(
        slice: &'a str, result: &mut Vec<InlineElement<'a>>,
        last_opener_star: &mut [Option<usize>; 3], last_opener_floor: &mut [Option<usize>; 3],
        mut is_beginning: bool,
    ) -> Vec<DelimiterStruct<'a>> {
        const ASCII_PUNCTUATION: [char; 27] = [
            '!', '"', '#', '%', '&', '\'', '(', ')', '*', ',', '.', '/', ':', ';', '?', '@', '[',
            '\\', ']', '^', '_', '`', '{', '}', '|', '~', '-',
        ];
        const UNICODE_WHITESPACE: [char; 25] = [
            '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0020}', '\u{0085}',
            '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}',
            '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}',
            '\u{2029}', '\u{202F}', '\u{205F}', '\u{3000}',
        ];
        let mut delimiter_stack: Vec<DelimiterStruct> = Vec::new();
        let mut two_spaces: bool = false;
        let mut is_space_stream: bool = false;
        let mut current: String = String::new();
        let mut html_current: String = String::new();
        let mut char_iter = slice.char_indices().peekable();
        let mut link_open: bool = false;
        let mut parse_link = true;
        let mut current_begin: i32 = -1;
        let mut current_link_text_slice: &str;
        let mut is_prev_punctuation: bool = false;
        let mut node;
        loop {
            match char_iter.next() {
                Some((start, '[')) => {
                    two_spaces = false;
                    is_space_stream = false;
                    if (!current.is_empty()) {
                        result.push(InlineElement {
                            element: Inline::Str(current.clone()),
                            slice: &slice[current_begin as usize..start],
                        });
                    }
                    current = String::new();
                    node = InlineElement {
                        slice: &slice[start..=start],
                        element: Inline::Temp(String::from('[')),
                    };
                    delimiter_stack.push(DelimiterStruct {
                        count: 0,
                        is_strong: false,
                        delimiter_char: '[',
                        delim_slice: &slice[start..=start],
                        typeof_delimiter: Potential::Opener,
                        temp_vec: vec![result.len()],
                    });
                    result.push(node);

                    link_open = true;
                },
                Some((start, ']')) => {
                    two_spaces = false;
                    is_space_stream = false;
                    if link_open {
                        let mut iter = delimiter_stack.iter().enumerate().rev();
                        let mut ending = false;
                        let mut closed = false;
                        node = InlineElement {
                            slice: &slice[start..=start],
                            element: Inline::Temp(String::from(']')),
                        };
                        if !current.is_empty() {
                            result.push(InlineElement {
                                element: Inline::Str(current.clone()),
                                slice: &slice[current_begin as usize..start],
                            });
                        }
                        result.push(node);
                        current = String::new();
                        if char_iter.peek().is_some_and(|(_x, y)| *y == '(') {
                            ending = true;
                        }
                        for i in (0..delimiter_stack.len()).rev() {
                            if delimiter_stack[i].delimiter_char == '['
                                && !delimiter_stack[i].delim_slice.is_empty()
                            {
                                if ending && closed {
                                    delimiter_stack[i].change_slice(&slice[start..start]);
                                } else if closed {
                                    break;
                                }
                                let mut upper =
                                    Self::change_to_base(delimiter_stack[i].delim_slice, slice);
                                current_link_text_slice = &slice[upper..=start];
                                delimiter_stack[i].change_slice(&slice[upper..upper]);
                                parse_link = true;
                                closed = true;
                            }
                        }
                    } else {
                        current.push(']');
                    }
                },
                Some((start, c @ ('*' | '_' | '~'))) => {
                    two_spaces = false;
                    let mut is_left_run = false;
                    let mut is_right_run = false;
                    let mut length = 1;
                    let mut is_strong = true;
                    let mut followed_by_punctuation = false;
                    let mut followed_by_whitespace = false;
                    let mut end_slice: usize = 0;
                    if !current.is_empty() {
                        // reset current after push
                        result.push(InlineElement {
                            element: Inline::Str(current.clone()),
                            slice: &slice[current_begin as usize..start],
                        });
                        current = String::new();
                    }
                    end_slice = start + 1;
                    loop {
                        if let Some(&(end, x)) = char_iter.peek() {
                            length += 1;
                            end_slice = end;
                            if c == x {
                                char_iter.next();
                                continue;
                            } else if ASCII_PUNCTUATION.contains(&x) {
                                followed_by_punctuation = true;
                                break;
                            } else if UNICODE_WHITESPACE.contains(&x) {
                                followed_by_whitespace = true;
                                break;
                            }

                            break;
                        }
                        if length > 1 {
                            end_slice += 1;
                        }
                        followed_by_whitespace = true;
                        break;
                    }
                    if end_slice - start == 1 {
                        is_strong = false;
                    }
                    current_begin = end_slice as i32;
                    if (end_slice - start > 2 && c == '~') {}
                    if !followed_by_whitespace
                        && (!followed_by_punctuation
                            || (followed_by_punctuation
                                && (is_space_stream || is_beginning || is_prev_punctuation)))
                        && end_slice != 0
                    {
                        is_left_run = true;
                        if last_opener_star[(end_slice - start) % 3].is_none() {
                            if c == '*' {
                                last_opener_star[(end_slice - start) % 3] =
                                    Some(delimiter_stack.len());
                            } else if c == '_' {
                                last_opener_floor[(end_slice - start) % 3] =
                                    Some(delimiter_stack.len());
                            }
                        }
                    }
                    if !(is_space_stream || is_beginning)
                        && (!is_prev_punctuation
                            || (is_prev_punctuation
                                && (followed_by_punctuation || followed_by_whitespace)))
                    {
                        is_right_run = true;
                    }
                    let mut text_nodes = Vec::new();
                    for i in start..end_slice {
                        let mut node = InlineElement {
                            element: Inline::Temp(String::from(c)),
                            slice: &slice[i..=i],
                        };
                        text_nodes.push(result.len());
                        result.push(node);
                    }
                    if is_left_run && is_right_run {
                        if c == '*' || (c == '_' && followed_by_punctuation && is_prev_punctuation)
                        {
                            delimiter_stack.push(DelimiterStruct {
                                count: slice[start..end_slice].len(),
                                is_strong,
                                delimiter_char: c,
                                delim_slice: &slice[start..end_slice],
                                typeof_delimiter: Potential::Both,
                                temp_vec: text_nodes,
                            });
                        } else if followed_by_punctuation {
                            delimiter_stack.push(DelimiterStruct {
                                count: slice[start..end_slice].len(),
                                is_strong,
                                delimiter_char: c,
                                delim_slice: &slice[start..end_slice],
                                typeof_delimiter: Potential::Closer,
                                temp_vec: text_nodes,
                            });
                        } else if is_prev_punctuation {
                            delimiter_stack.push(DelimiterStruct {
                                count: slice[start..end_slice].len(),
                                is_strong,
                                delimiter_char: c,
                                delim_slice: &slice[start..end_slice],
                                typeof_delimiter: Potential::Opener,
                                temp_vec: text_nodes,
                            });
                        }
                    } else if is_left_run {
                        delimiter_stack.push(DelimiterStruct {
                            count: slice[start..end_slice].len(),
                            is_strong,
                            delimiter_char: c,
                            delim_slice: &slice[start..end_slice],
                            typeof_delimiter: Potential::Opener,
                            temp_vec: text_nodes,
                        });
                    } else if is_right_run {
                        delimiter_stack.push(DelimiterStruct {
                            count: slice[start..end_slice].len(),
                            is_strong,
                            delimiter_char: c,
                            delim_slice: &slice[start..end_slice],
                            typeof_delimiter: Potential::Closer,
                            temp_vec: text_nodes,
                        });
                    }
                    is_prev_punctuation = true;
                    is_space_stream = false;
                },
                Some((i, '\\')) => {
                    is_space_stream = false;
                    two_spaces = false;
                    if let Some((_, peek_char)) = char_iter.next() {
                        if !ASCII_PUNCTUATION.contains(&peek_char) {
                            current.push('\\');
                            is_prev_punctuation = true;
                        }
                        if peek_char == '\n' {
                            result.push(InlineElement {
                                element: Inline::LineBreak,
                                slice: &slice[i..=i],
                            });
                            continue;
                        }
                        current.push(peek_char);
                    }
                },
                Some((index, '&')) => {
                    is_prev_punctuation = false;
                    two_spaces = false;
                    let begin_index = index;
                    html_current.push('&');
                    is_space_stream = false;
                    match char_iter.peek() {
                        Some((_, '#')) => {
                            html_current.push('#');
                            char_iter.next();
                            match char_iter.peek() {
                                Some((_, c @ ('X' | 'x'))) => {
                                    html_current.push(*c);
                                    char_iter.next();
                                    let mut parse_result: StringOrChar;
                                    (parse_result, char_iter) =
                                        Self::parse_hex_entity(char_iter.clone());
                                    match parse_result {
                                        StringOrChar::NoHTMLString(_) => {
                                            current.push_str(&html_current);
                                        },
                                        StringOrChar::HTMLChar(c) => {
                                            current.push(c);
                                        },
                                    }
                                },
                                Some((..)) => {
                                    Self::parse_dec_entity(char_iter.clone());
                                    let mut parse_result: StringOrChar;
                                    (parse_result, char_iter) =
                                        Self::parse_dec_entity(char_iter.clone());
                                    match parse_result {
                                        StringOrChar::NoHTMLString(_) => {
                                            current.push_str(&html_current);
                                        },
                                        StringOrChar::HTMLChar(c) => {
                                            if current_begin == -1 {
                                                current_begin = begin_index as i32;
                                            }
                                            current.push(c);
                                        },
                                    }
                                },
                                None => {
                                    current.push_str(&html_current);
                                },
                            }
                            html_current = String::new();
                        },
                        Some((..)) => {},
                        None => {},
                    }
                },
                Some((i, '\n')) => {
                    is_space_stream = true;
                    if !current.is_empty() {
                        result.push(InlineElement {
                            element: Inline::Str(current),
                            slice: &slice[current_begin as usize..=i],
                        });
                        current = String::new();
                        current_begin = i as i32;
                    }
                    if let Some(x) = result.pop() {
                        if x.element != Inline::Space {
                            result.push(x);
                        }
                        result.push(InlineElement {
                            element: Inline::SoftBreak,
                            slice: &slice[i..=i],
                        });
                    }
                },
                Some((index, c)) if UNICODE_WHITESPACE.contains(&c) => {
                    is_prev_punctuation = false;
                    two_spaces = false;
                    if c == ' ' {
                        while let Some(&(end, y)) = char_iter.peek() {
                            if y == ' ' {
                                char_iter.next();
                                two_spaces = true;
                            } else if y == '\n' && two_spaces {
                                result.push(InlineElement {
                                    element: Inline::LineBreak,
                                    slice: &slice[index..end],
                                });
                                break;
                            } else {
                                break;
                            }
                        }
                    }
                    if !is_space_stream {
                        if !current.is_empty() {
                            result.push(InlineElement {
                                element: Inline::Str(current),
                                slice: &slice[current_begin as usize..index],
                            });
                        }
                        result.push(InlineElement {
                            element: Inline::Space,
                            slice: &slice[index..index + c.len_utf8()],
                        });
                        current = String::new();
                        is_space_stream = true;
                    }
                    if parse_link {
                        parse_link = false;
                    }
                },
                Some((index, c)) => {
                    if (ASCII_PUNCTUATION.contains(&c)) {
                        is_prev_punctuation = true;
                    } else {
                        is_prev_punctuation = false;
                    }
                    is_space_stream = false;
                    two_spaces = false;
                    if current_begin == -1 {
                        current_begin = index as i32;
                    }
                    current.push(c);
                },
                None => {
                    if current.len() != 0 {
                        result.push(InlineElement {
                            element: Inline::Str(current),
                            slice: &slice[current_begin as usize..slice.len()],
                        });
                    }
                    break;
                },
            }
            is_beginning = false;
        }
        return delimiter_stack;
    }

    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn temp_method(lines: &[String]) -> Vec<Inline> {
        // todo
        let mut result = Vec::new();
        let ascii_punctuation = [
            '!', '"', '#', '%', '&', '\'', '(', ')', '*', ',', '.', '/', ':', ';', '?', '@', '[',
            '\\', ']', '^', '_', '`', '{', '}', '|', '~',
        ];
        let mut space = false;
        let mut backtickStringLength: u32 = 0;
        let mut codeSpanActive: bool = false;
        let mut current = String::new();
        let mut html_entity_state = HtmlEntityState::NoState;
        for l in lines {
            let mut char_iter = l.trim().char_indices().peekable();
            let char_line: &str = l.trim();
            let mut start_slice_index: usize = 0;
            let mut length: u32 = 0;
            loop {
                // Space remover
                match char_iter.peek() {
                    Some((_, '\t' | ' ')) => {},
                    Some(_) => space = false,
                    None => {},
                }

                // HtmlEntityValue check
                if matches!(html_entity_state, HtmlEntityState::Dec | HtmlEntityState::Hex) {
                    match char_iter.next() {
                        Some((end_index, ';')) => {
                            length = 0;
                            let entity_value = u32::from_str_radix(
                                &l.trim()[start_slice_index..end_index],
                                html_entity_state.get_base(),
                            );
                            match entity_value {
                                Ok(value) => current.push(char::from_u32(value).unwrap()),
                                Err(_) => current.push(char::from_u32(0xfffd).unwrap()),
                            }
                            html_entity_state = HtmlEntityState::NoState;
                        },
                        Some((_, _c @ ('0'..='9')))
                            if length < html_entity_state.get_entity_max_length() =>
                        {
                            length += 1;
                        },
                        Some((_, ('a'..='f' | 'A'..='F')))
                            if html_entity_state.get_base() == 16 =>
                        {
                            if length < html_entity_state.get_entity_max_length() {
                                length += 1;
                            }
                        },
                        // This could be written better I think
                        Some((_, ('x' | 'X'))) if html_entity_state.get_base() == 16 => {
                            continue;
                        },

                        Some((end_index, _)) => {
                            html_entity_state = HtmlEntityState::NoState;
                            length = 0;
                            current.push_str(&char_line[start_slice_index..end_index]);
                        },
                        None => {},
                    }
                    continue;
                }
                match char_iter.next() {
                    Some((backStringIndex, '`')) if !codeSpanActive => {
                        backtickStringLength = 1;
                        let mut onlySpace = true;
                        let mut start_slice_index = 0;
                        while let Some((index, c)) = char_iter.next() {
                            if c == '`' && !codeSpanActive {
                                backtickStringLength += 1;
                            } else if c != '`' && !codeSpanActive {
                                codeSpanActive = true;
                                start_slice_index = index;
                            } else if c == '`'
                                && codeSpanActive
                                && char_line[backStringIndex..start_slice_index]
                                    == char_line[index..index + backtickStringLength as usize]
                            {
                                let tmp = attr_empty();
                                for _ in 0..backtickStringLength - 1 {
                                    char_iter.next();
                                }
                                if let Some((_, x)) = char_iter.peek() {
                                    if *x == '`' {
                                        onlySpace = false;
                                        char_iter.next();
                                        continue;
                                    }
                                }
                                if !onlySpace
                                    && char_line[start_slice_index..index].starts_with(' ')
                                    && char_line[start_slice_index..index].ends_with(' ')
                                    && char_line[start_slice_index..index].len() != 1
                                {
                                    result.push(Inline::Code(
                                        tmp,
                                        char_line[start_slice_index + 1..index - 1].to_string(),
                                    ));
                                    break;
                                }
                                result.push(Inline::Code(
                                    tmp,
                                    char_line[start_slice_index..index].to_string(),
                                ));
                                break;
                            } else if c != ' ' && codeSpanActive {
                                onlySpace = false;
                            }
                        }
                    },
                    Some((_, ' ' | '\t')) if !space => {
                        result.push(Inline::Str(current));
                        current = String::new();
                        result.push(Inline::Space);
                        space = true;
                    },
                    Some((_, ' ' | '\t')) if space => {},
                    Some((_, '\\')) =>
                        if let Some((_, c)) = char_iter.next() {
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
                    Some((pos, '&')) => {
                        start_slice_index = pos;
                        match char_iter.peek() {
                            Some((_, '#')) => {
                                char_iter.next();
                                match char_iter.peek() {
                                    Some((index, ('X' | 'x'))) => {
                                        html_entity_state = HtmlEntityState::Hex;
                                        start_slice_index = *index + 1;
                                    },
                                    Some((index, _)) => {
                                        html_entity_state = HtmlEntityState::Dec;
                                        start_slice_index = *index;
                                    },
                                    None => {},
                                }
                            },
                            Some(_) => {
                                todo!()
                            },
                            None => {},
                        }
                    },
                    Some((_, c)) => {
                        current.push(c);
                    },
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

    #[must_use]
    pub fn parse_vector(vector: &[String]) -> Vec<Inline> { Self::parse_lines(&vector.join("")) }

    pub fn parse_line(line: String) -> Vec<Inline> { Self::parse_lines(&line) }

    fn _parse_one_line(line: &str, result: &mut Vec<Inline>) {
        // todo
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

#[allow(dead_code)]
#[allow(clippy::too_many_lines)]
fn parse_emph<'a>(
    base_string: &'a str, delimiter_stack: &mut Vec<DelimiterStruct<'a>>,
    last_opener_star: &[Option<usize>; 3], last_opener_floor: &[Option<usize>; 3],
    stack_bottom: usize, result_vec: &mut Vec<InlineElement<'a>>,
) -> Vec<InlineElement<'a>> {
    let mut emph_vector: Vec<InlineElement> = Vec::new();
    let mut remove_vec_indices: Vec<usize> = Vec::new();
    // Nie moge przepisać na iterator bo iterator borrowuje wartość delimiter_stack
    for index in 0..delimiter_stack.len() {
        let mut delim = delimiter_stack[index].clone();
        match delim.typeof_delimiter {
            Potential::Opener | Potential::None => {},
            Potential::Both | Potential::Closer => {
                let length = delim.count;
                let mut min_index = 0;
                if delim.delimiter_char == '*' {
                    if let Some(c) = last_opener_star[(length + 1) % 3] {
                        min_index = c;
                    }
                    if let Some(c) = last_opener_star[(length + 2) % 3]
                        && min_index > c
                    {
                        min_index = c;
                    }
                } else {
                    if let Some(c) = last_opener_floor[(length + 1) % 3] {
                        min_index = c;
                    }
                    if let Some(c) = last_opener_floor[(length + 2) % 3]
                        && min_index < c
                    {
                        min_index = c;
                    }
                }
                if index == 0 {
                    continue;
                }
                let mut last_index: usize;
                for j in (0..=index - 1).rev() {
                    if !matches!(delimiter_stack[j].typeof_delimiter, Potential::Closer)
                        && ((matches!(delimiter_stack[j].typeof_delimiter, Potential::Both)
                            || matches!(delim.typeof_delimiter, Potential::Both))
                            && (delimiter_stack[j].count + length) % 3 != 0
                            || (length % 3 == 0 && delimiter_stack[j].delim_slice.len() % 3 == 0))
                        || (matches!(delimiter_stack[j].typeof_delimiter, Potential::Opener)
                            && matches!(delim.typeof_delimiter, Potential::Closer))
                            && delimiter_stack[j].delimiter_char == delim.delimiter_char
                    {
                        let lower_bound = delimiter_stack[j].delim_slice.as_ptr() as usize
                            + delimiter_stack[j].delim_slice.len()
                            - base_string.as_ptr() as usize;
                        if lower_bound < stack_bottom {
                            continue;
                        }
                        while delim.delim_slice.len() >= 2
                            && delimiter_stack[j].delim_slice.len() >= 2
                        {
                            let upper_bound = delim.delim_slice.as_ptr() as usize
                                - base_string.as_ptr() as usize
                                + 2;
                            if (lower_bound < stack_bottom || upper_bound < stack_bottom) {
                                break;
                            }
                            result_vec[delimiter_stack[j].temp_vec.pop().unwrap()] =
                                InlineElement { element: Inline::None, slice: "" };
                            const SPECIAL_CHARS: [char; 5] = ['[', ']', '~', '*', '_'];
                            let mut lower_res_index = delimiter_stack[j].temp_vec.pop().unwrap();
                            result_vec[lower_res_index] =
                                InlineElement { element: Inline::None, slice: "" };
                            result_vec[delim.temp_vec.remove(0)] =
                                InlineElement { element: Inline::None, slice: "" };
                            let mut upper_res_index = delim.temp_vec.remove(0);
                            result_vec[upper_res_index] =
                                InlineElement { element: Inline::None, slice: "" };
                            let mut nested_inlines = Vec::new();
                            let mut is_last_str = false;
                            for x in lower_res_index..=upper_res_index {
                                match &result_vec[x].element {
                                    Inline::Temp(c) => {
                                        if is_last_str {
                                            let mut temp = nested_inlines.pop().unwrap();
                                            if let Inline::Str(x) = temp {
                                                nested_inlines.push(Inline::Str(x + c));
                                            }
                                        } else {
                                            nested_inlines.push(Inline::Str(c.to_string()));
                                            is_last_str = true;
                                        }
                                        result_vec[x] =
                                            InlineElement { element: Inline::None, slice: "" };
                                    },
                                    Inline::None =>
                                        result_vec[x] =
                                            InlineElement { element: Inline::None, slice: "" },
                                    Inline::Str(c) => {
                                        if is_last_str {
                                            let mut temp = nested_inlines.pop().unwrap();
                                            if let Inline::Str(x) = temp {
                                                nested_inlines.push(Inline::Str(x + c));
                                            }
                                        } else {
                                            is_last_str = true;
                                            nested_inlines.push(result_vec[x].element.clone());
                                        }
                                        result_vec[x] =
                                            InlineElement { element: Inline::None, slice: "" };
                                    },
                                    _ => {
                                        nested_inlines.push(result_vec[x].element.clone());
                                        result_vec[x] =
                                            InlineElement { element: Inline::None, slice: "" };
                                        is_last_str = false;
                                    },
                                }
                            }
                            result_vec[lower_res_index] = InlineElement {
                                element: Inline::Strong(nested_inlines.clone()),
                                slice: &base_string[lower_bound..upper_bound],
                            };
                            emph_vector.push(InlineElement {
                                element: Inline::Strong(nested_inlines),
                                slice: &base_string[lower_bound..upper_bound],
                            });
                            let bottom_index =
                                lower_bound + 2 - delimiter_stack[j].delim_slice.len();
                            delimiter_stack[j]
                                .change_slice(&base_string[bottom_index..lower_bound]);
                            let top_index = upper_bound - 2 + delim.delim_slice.len();
                            delim.change_slice(&base_string[upper_bound..top_index]);
                            for y in j + 1..index {
                                delimiter_stack[y].change_slice("");
                                delimiter_stack[y].typeof_delimiter = Potential::None;
                                delimiter_stack[y].delimiter_char = '-';
                                delimiter_stack[y].count = 0;
                            }
                        }
                        if !delim.delim_slice.is_empty()
                            && !delimiter_stack[j].delim_slice.is_empty()
                            && delim.delimiter_char == delimiter_stack[j].delimiter_char
                        {
                            let mut lower_res_index = delimiter_stack[j].temp_vec.pop().unwrap();
                            let mut upper_res_index = delim.temp_vec.remove(0);
                            result_vec[upper_res_index] =
                                InlineElement { element: Inline::None, slice: "" };
                            result_vec[lower_res_index] =
                                InlineElement { element: Inline::None, slice: "" };
                            let mut nested_inlines = Vec::new();
                            let mut is_last_str: bool = false;
                            for x in lower_res_index..=upper_res_index {
                                match &result_vec[x].element {
                                    Inline::Temp(c) => {
                                        if is_last_str {
                                            let mut temp = nested_inlines.pop().unwrap();
                                            if let Inline::Str(x) = temp {
                                                nested_inlines.push(Inline::Str(x + c));
                                            }
                                        } else {
                                            nested_inlines.push(Inline::Str(c.to_string()));
                                            is_last_str = true;
                                        }
                                        result_vec[x] =
                                            InlineElement { element: Inline::None, slice: "" };
                                    },
                                    Inline::None =>
                                        result_vec[x] =
                                            InlineElement { element: Inline::None, slice: "" },
                                    Inline::Str(c) => {
                                        if is_last_str {
                                            let mut temp = nested_inlines.pop().unwrap();
                                            if let Inline::Str(x) = temp {
                                                nested_inlines.push(Inline::Str(x + c));
                                            }
                                        } else {
                                            is_last_str = true;
                                            nested_inlines.push(result_vec[x].element.clone());
                                        }
                                        result_vec[x] =
                                            InlineElement { element: Inline::None, slice: "" };
                                    },
                                    _ => {
                                        nested_inlines.push(result_vec[x].element.clone());
                                        result_vec[x] =
                                            InlineElement { element: Inline::None, slice: "" };
                                        is_last_str = false;
                                    },
                                }
                            }

                            let lower_bound = delimiter_stack[j].delim_slice.as_ptr() as usize
                                - base_string.as_ptr() as usize
                                + delimiter_stack[j].delim_slice.len()
                                - 1;
                            let upper_bound = delim.delim_slice.as_ptr() as usize
                                - base_string.as_ptr() as usize
                                + 1;
                            result_vec[lower_res_index] = InlineElement {
                                element: Inline::Emph(nested_inlines.clone()),
                                slice: &base_string[lower_bound..upper_bound],
                            };
                            emph_vector.push(InlineElement {
                                element: Inline::Emph(nested_inlines),
                                slice: &base_string[lower_bound..upper_bound],
                            });
                            let bottom_index =
                                lower_bound + 1 - delimiter_stack[j].delim_slice.len();
                            delimiter_stack[j]
                                .change_slice(&base_string[bottom_index..lower_bound]);
                            let top_index = upper_bound - 1 + delim.delim_slice.len();
                            delim.change_slice(&base_string[upper_bound..top_index]);
                            for y in j + 1..index {
                                delimiter_stack[y].change_slice("");
                                delimiter_stack[y].typeof_delimiter = Potential::None;
                                delimiter_stack[y].delimiter_char = '-';
                                delimiter_stack[y].count = 0;
                            }
                        }

                        if matches!(delimiter_stack[j].typeof_delimiter, Potential::Opener) {
                        } else if matches!(delimiter_stack[j].typeof_delimiter, Potential::Both) {
                        }
                    }
                }
                // for x in result_vec.clone() {
                //     if x.element != Inline::None {
                //         print!("{:?}\n", x.element);
                //     }
                // }
                delimiter_stack[index] = delim;
            },
        }
    }
    emph_vector
}

// #[cfg(test)]
// mod test {
//     use std::fmt::Debug;
//
//     use crate::ast::attr_empty;
//     use crate::inline_parser::*;
//
//     #[test]
//     fn test_test() {
//         // let result = MdReader::read("> ```\n> aaa\n\nbbb").into_ok();
//         let test = vec!["hello        rust \\' \\ab".to_string()];
//         let result = InlineParser::parse_lines(&test);
//         assert_eq!(Inline::Str("hello".to_string()), result[0]);
//         assert_eq!(Inline::Space, result[1]);
//         assert_eq!(Inline::Str("rust".to_string()), result[2]);
//         assert_eq!(Inline::Space, result[3]);
//         assert_eq!(Inline::Str("'".to_string()), result[4]);
//         assert_eq!(Inline::Space, result[5]);
//     }
//
//     #[test]
//     fn html_entity_dec_test() {
//         let test = vec!["&#42;  asdfsasdasdasffs".to_string()];
//         let result = InlineParser::parse_lines(&test);
//         let Inline::Str(s) = &result[0] else { return };
//         assert_eq!(s.to_string(), String::from("*"));
//         assert_eq!(Inline::Space, result[1]);
//         let Inline::Str(s) = &result[2] else { return };
//         assert_eq!(s.to_string(), String::from("asdfsasdasdasffs"));
//     }
//
//     #[test]
//     fn html_entity_hex_test() {
//         let test = vec!["&#x2A;  asdfsasdasdasffsasdf".to_string()];
//         let result = InlineParser::parse_lines(&test);
//         let Inline::Str(s) = &result[0] else { return };
//         assert_eq!(s.to_string(), String::from("*"));
//         assert_eq!(Inline::Space, result[1]);
//         let Inline::Str(s) = &result[2] else { return };
//         assert_eq!(s.to_string(), String::from("asdfsasdasdasffsasdf"));
//     }
//
//     #[test]
//     fn code_span_test() {
//         let test = vec!["``` abc ```".to_string()];
//         let result = InlineParser::parse_lines(&test);
//         let tmp = attr_empty();
//         let Inline::Code(tmp, s) = &result[0] else {
//             return;
//         };
//         assert_eq!(s.to_string(), String::from("abc"));
//     }
// }
