use std::collections::HashMap;
use std::fs;
use std::iter::Peekable;
use std::num::ParseIntError;
use std::str::CharIndices;
use std::string::String;

use lazy_static::lazy_static;

use crate::ast::{attr_empty, Inline};

/// Structure containing methods for passing inlines with the main method for this being
/// [`Self::parse_lines()`]
pub struct InlineParser;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Potential {
    Opener,
    Closer,
    Both,
    None,
}

#[derive(Clone, Debug)]
struct DelimiterStruct<'a> {
    count: usize,
    delimiter_char: char,
    delim_slice: &'a str,
    typeof_delimiter: Potential,
    temp_vec: Vec<usize>,
}

impl<'a> DelimiterStruct<'a> {
    fn change_slice(&mut self, new_slice: &'a str) { self.delim_slice = new_slice; }
}

// Może zmienić an Inline
#[derive(Clone)]
struct InlineElement<'a> {
    slice: &'a str,
    element: Inline,
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
        let vec: Vec<(String, String)> =
            serde_json::from_str(&fs::read_to_string("entities.json").unwrap()).unwrap();
        vec.into_iter().collect()
    };
}

impl InlineParser {
    const ASCII_PUNCTUATION: [char; 31] = [
        '!', '"', '#', '%', '&', '\'', '(', ')', '*', ',', '.', '/', ':', ';', '?', '@', '[', '\\',
        ']', '^', '_', '`', '{', '}', '|', '~', '-', '$', '<', '>', '=',
    ];
    const UNICODE_WHITESPACE: [char; 25] = [
        '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0020}', '\u{0085}',
        '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}',
        '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}',
        '\u{2029}', '\u{202F}', '\u{205F}', '\u{3000}',
    ];

    fn get_backtick_string_length_vector(paragraph: &str) -> Vec<BacktickString> {
        let mut iter = paragraph.char_indices();
        let mut result = Vec::new();
        let mut prev_escape = false;
        let mut count_map: HashMap<usize, usize> = HashMap::new();
        loop {
            match iter.next() {
                Some((_, '\\')) => {
                    prev_escape = true;
                },
                Some((s, '`')) => loop {
                    match iter.next() {
                        Some((_, '`')) => continue,
                        Some((e, _)) => {
                            if prev_escape && count_map.get(&(e - s)).is_some_and(|x| x % 2 == 1) {
                                prev_escape = false;
                                break;
                            }
                            result.push(BacktickString { backtick_length: e - s, start_index: s });
                            if let Some(x) = count_map.get(&(e - s)) {
                                count_map.insert(e - s, x + 1);
                            }
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
                Some(_) => {
                    prev_escape = false;
                    continue;
                },
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

    fn parse_backtick_string_length_vector<'a>(
        paragraph: &'a str, backtick_vec: &[BacktickString],
    ) -> Vec<SliceVariant<'a>> {
        let mut open_iter = backtick_vec.iter();
        let mut prev_index = 0;
        let mut result: Vec<SliceVariant> = Vec::new();
        loop {
            if let Some(c) = open_iter.next() {
                if c.start_index != 0 {
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
                if prev_index != paragraph.len() {
                    result.push(SliceVariant::InlineSlice(&paragraph[prev_index..paragraph.len()]));
                }
                return result;
            }
        }
    }

    fn parse_code_spans(paragraph: &str) -> Vec<SliceVariant> {
        let backticks: Vec<BacktickString> = Self::get_backtick_string_length_vector(paragraph);
        Self::parse_backtick_string_length_vector(paragraph, &backticks)
    }

    /// This function takes a text slice and proceeds to parse every html entity containing
    /// abbreviated char names for example &quot; will be parsed to "
    #[must_use]
    pub fn parse_html_entities(paragraph: &str) -> String {
        let mut chars = paragraph.chars();
        let mut new_paragraph = String::new();
        let mut current;
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
                                new_paragraph.push_str(&current);
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

    /// This function iterates over the given paragraph and runs methods when it finds special
    /// characters having some functionality in GFM
    #[must_use]
    pub fn parse_lines(paragraph: &str) -> Vec<Inline> {
        let new_paragraph = Self::parse_html_entities(paragraph);
        let inlines_and_code = Self::parse_code_spans(&new_paragraph);
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

        Self::parse_emph(&new_paragraph, &mut delimiter_stack, 0, &mut result);

        for x in &result {
            match x.element.clone() {
                Inline::Str(c) | Inline::Temp(c) =>
                    if is_prev_str {
                        let temp = true_result.pop().unwrap();
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
        // for x in &true_result {
        //     if *x != Inline::None {
        //         print!("{:?} ", x);
        //     }
        // }
        true_result
    }

    fn parse_code_slice(slice: &str) -> InlineElement {
        let mut x = 0;
        while slice[x..slice.len() - x].starts_with('`') && slice[x..slice.len() - x].ends_with('`')
        {
            x += 1;
        }
        let result = slice[x..slice.len() - x].replace('\n', " ");
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
                Some((_, ';')) =>
                    if !current_bonus.is_empty() {
                        let entity_value = u32::from_str_radix(&current_bonus, 16);
                        return Self::safe_entity_parse(&entity_value, copy_iter.clone());
                    },
                Some((..)) => return (StringOrChar::NoHTMLString(current_bonus), begin_iter),
                None => {},
            }
            length += 1;
        }
        (StringOrChar::NoHTMLString(current_bonus), begin_iter)
    }

    fn safe_entity_parse<'a>(
        entity_value: &Result<u32, ParseIntError>, mut copy_iter: Peekable<CharIndices<'a>>,
    ) -> (StringOrChar, Peekable<CharIndices<'a>>) {
        match entity_value {
            Ok(x) => {
                copy_iter.next();
                (StringOrChar::HTMLChar(char::from_u32(*x).unwrap()), copy_iter)
            },
            Err(_) => (StringOrChar::HTMLChar(char::from_u32(0xfffd).unwrap()), copy_iter),
        }
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
                Some((_, ';')) =>
                    if !current_bonus.is_empty() {
                        let entity_value = current_bonus.parse::<u32>();
                        return Self::safe_entity_parse(&entity_value, copy_iter.clone());
                    },
                Some(_) | None => return (StringOrChar::NoHTMLString(current_bonus), begin_iter),
            }
            length += 1;
        }
        (StringOrChar::NoHTMLString(current_bonus), begin_iter)
    }

    fn change_to_base(slice1: &str, slice2: &str) -> usize {
        slice1.as_ptr() as usize - slice2.as_ptr() as usize
    }

    #[allow(clippy::too_many_lines)]
    fn parse_inline_slice<'a>(
        slice: &'a str, result: &mut Vec<InlineElement<'a>>,
        last_opener_star: &mut [Option<usize>; 3], last_opener_floor: &mut [Option<usize>; 3],
        mut is_beginning: bool,
    ) -> Vec<DelimiterStruct<'a>> {
        let mut delimiter_stack: Vec<DelimiterStruct> = Vec::new();
        let mut is_space_stream: bool = false;
        let mut current: String = String::new();
        let mut html_current: String = String::new();
        let mut char_iter = slice.char_indices().peekable();
        let mut link_open: bool = false;
        let mut parse_link = true;
        let mut current_begin: Option<usize> = Some(0);
        let mut is_prev_punctuation: bool = false;

        while let Some((start, c)) = char_iter.next() {
            match c {
                '[' => Self::handle_open_bracket(
                    slice, result, &mut current, &current_begin, &mut delimiter_stack, start,
                    &mut link_open,
                ),
                ']' => Self::handle_close_bracket(
                    slice, result, &mut current, &current_begin, &mut delimiter_stack, start,
                    link_open, &mut parse_link, &mut char_iter,
                ),
                '*' | '_' | '~' => Self::handle_special_char(
                    slice, result, &mut current, &mut current_begin, &mut char_iter, c, start,
                    &mut delimiter_stack, last_opener_star, last_opener_floor,
                    &mut is_prev_punctuation, &mut is_space_stream, is_beginning,
                ),
                '\\' => Self::handle_backslash(
                    slice, result, &mut current, &mut current_begin, &mut char_iter, start,
                    &mut is_prev_punctuation,
                ),
                '&' => Self::handle_ampersand(&mut current, &mut char_iter, &mut html_current),
                '\n' => Self::handle_newline(
                    slice, result, &mut current, &mut current_begin, start, &mut is_space_stream,
                ),
                c if Self::UNICODE_WHITESPACE.contains(&c) => Self::handle_whitespace(
                    slice, result, &mut current, &current_begin, &mut char_iter,
                    &mut is_space_stream, c, start,
                ),
                c => Self::handle_regular_char(
                    c, &mut current, &mut current_begin, start, &mut is_prev_punctuation,
                    &mut is_space_stream,
                ),
            }
            is_beginning = false;
        }

        if !current.is_empty() {
            result.push(InlineElement {
                element: Inline::Str(current),
                slice: &slice[current_begin.unwrap()..slice.len()],
            });
        }

        delimiter_stack
    }

    fn handle_open_bracket<'a>(
        slice: &'a str, result: &mut Vec<InlineElement<'a>>, current: &mut String,
        current_begin: &Option<usize>, delimiter_stack: &mut Vec<DelimiterStruct<'a>>,
        start: usize, link_open: &mut bool,
    ) {
        if !current.is_empty() {
            result.push(InlineElement {
                element: Inline::Str(current.clone()),
                slice: &slice[current_begin.unwrap()..start],
            });
        }
        *current = String::new();
        let node = InlineElement {
            slice: &slice[start..=start],
            element: Inline::Temp(String::from('[')),
        };
        delimiter_stack.push(DelimiterStruct {
            count: 0,
            delimiter_char: '[',
            delim_slice: &slice[start..=start],
            typeof_delimiter: Potential::Opener,
            temp_vec: vec![result.len()],
        });
        result.push(node);
        *link_open = true;
    }

    fn handle_close_bracket<'a>(
        slice: &'a str, result: &mut Vec<InlineElement<'a>>, current: &mut String,
        current_begin: &Option<usize>, delimiter_stack: &mut [DelimiterStruct<'a>], start: usize,
        link_open: bool, parse_link: &mut bool, char_iter: &mut Peekable<CharIndices<'a>>,
    ) {
        if link_open {
            // let iter = delimiter_stack.iter().enumerate().rev();
            let mut ending = false;
            let mut closed = false;
            let node = InlineElement {
                slice: &slice[start..=start],
                element: Inline::Temp(String::from(']')),
            };
            if !current.is_empty() {
                result.push(InlineElement {
                    element: Inline::Str(current.clone()),
                    slice: &slice[current_begin.unwrap()..start],
                });
            }
            result.push(node);
            *current = String::new();
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
                    let upper = Self::change_to_base(delimiter_stack[i].delim_slice, slice);
                    delimiter_stack[i].change_slice(&slice[upper..upper]);
                    *parse_link = true;
                    closed = true;
                }
            }
        } else {
            current.push(']');
        }
    }

    fn handle_special_char<'a>(
        slice: &'a str, result: &mut Vec<InlineElement<'a>>, current: &mut String,
        current_begin: &mut Option<usize>, char_iter: &mut Peekable<CharIndices<'a>>, c: char,
        start: usize, delimiter_stack: &mut Vec<DelimiterStruct<'a>>,
        last_opener_star: &mut [Option<usize>; 3], last_opener_floor: &mut [Option<usize>; 3],
        is_prev_punctuation: &mut bool, is_space_stream: &mut bool, is_beginning: bool,
    ) {
        if !current.is_empty() {
            result.push(InlineElement {
                element: Inline::Str(current.clone()),
                slice: &slice[current_begin.unwrap()..start],
            });
            *current = String::new();
        }

        let mut is_left_run = false;
        let mut is_right_run = false;
        let mut length = 1;
        let mut followed_by_punctuation = false;
        let mut followed_by_whitespace = false;
        let mut end_slice = start + 1;

        loop {
            if let Some(&(end, x)) = char_iter.peek() {
                length += 1;
                end_slice = end;
                if c == x {
                    char_iter.next();
                    continue;
                } else if Self::ASCII_PUNCTUATION.contains(&x) {
                    followed_by_punctuation = true;
                    break;
                } else if Self::UNICODE_WHITESPACE.contains(&x) {
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

        *current_begin = Some(end_slice);

        if !followed_by_whitespace
            && (!followed_by_punctuation
            || (*is_space_stream || is_beginning || *is_prev_punctuation))
            && end_slice != 0
        {
            is_left_run = true;
            if last_opener_star[(end_slice - start) % 3].is_none() {
                if c == '*' {
                    last_opener_star[(end_slice - start) % 3] = Some(delimiter_stack.len());
                } else if c == '_' {
                    last_opener_floor[(end_slice - start) % 3] = Some(delimiter_stack.len());
                }
            }
        }

        if !(*is_space_stream || is_beginning)
            && (!*is_prev_punctuation || followed_by_punctuation || followed_by_whitespace)
        {
            is_right_run = true;
        }

        let mut text_nodes = Vec::new();
        for i in start..end_slice {
            let node =
                InlineElement { element: Inline::Temp(String::from(c)), slice: &slice[i..=i] };
            text_nodes.push(result.len());
            result.push(node);
        }
        if end_slice - start > 2 && c == '~' {
            return;
        }
        let typeof_delimiter = if is_left_run && is_right_run {
            if c == '*' || (c == '_' && followed_by_punctuation && *is_prev_punctuation) || c == '~'
            {
                Potential::Both
            } else if followed_by_punctuation {
                Potential::Closer
            } else if *is_prev_punctuation {
                Potential::Opener
            } else {
                Potential::None
            }
        } else if is_left_run {
            Potential::Opener
        } else if is_right_run {
            Potential::Closer
        } else {
            Potential::None
        };

        delimiter_stack.push(DelimiterStruct {
            count: slice[start..end_slice].len(),
            delimiter_char: c,
            delim_slice: &slice[start..end_slice],
            typeof_delimiter,
            temp_vec: text_nodes,
        });

        *is_prev_punctuation = true;
        *is_space_stream = false;
    }

    fn handle_backslash<'a>(
        slice: &'a str, result: &mut Vec<InlineElement<'a>>, current: &mut String,
        mut current_begin: &mut Option<usize>, char_iter: &mut Peekable<CharIndices<'a>>,
        start: usize, is_prev_punctuation: &mut bool,
    ) {
        if let Some((_, peek_char)) = char_iter.next() {
            if !Self::ASCII_PUNCTUATION.contains(&peek_char) {
                current.push('\\');
                *is_prev_punctuation = true;
            }
            if peek_char == '\n' {
                current.pop();
                if !current.is_empty() {
                    result.push(InlineElement {
                        element: Inline::Str((*current).to_string()),
                        slice: &slice[current_begin.unwrap()..start],
                    });
                    *current_begin = Some(start);
                    *current = String::new();
                }
                result.push(InlineElement {
                    element: Inline::LineBreak,
                    slice: &slice[start..=start],
                });
                return;
            }
            current.push(peek_char);
        }
    }

    fn handle_ampersand(
        current: &mut String, char_iter: &mut Peekable<CharIndices>, html_current: &mut String,
    ) {
        html_current.push('&');
        if let Some((_, '#')) = char_iter.peek() {
            html_current.push('#');
            char_iter.next();
            if let Some((_, c @ ('X' | 'x'))) = char_iter.peek() {
                html_current.push(*c);
                char_iter.next();
                let parse_result = Self::parse_hex_entity(char_iter.clone());
                match parse_result.0 {
                    StringOrChar::NoHTMLString(_) => {
                        current.push_str(html_current);
                    },
                    StringOrChar::HTMLChar(c) => {
                        current.push(c);
                    },
                }
                *char_iter = parse_result.1;
            } else {
                let parse_result = Self::parse_dec_entity(char_iter.clone());
                match parse_result.0 {
                    StringOrChar::NoHTMLString(_) => {
                        current.push_str(html_current);
                    },
                    StringOrChar::HTMLChar(c) => {
                        current.push(c);
                    },
                }
                *char_iter = parse_result.1;
            }
            *html_current = String::new();
        }
    }

    fn handle_newline<'a>(
        slice: &'a str, result: &mut Vec<InlineElement<'a>>, current: &mut String,
        current_begin: &mut Option<usize>, start: usize, is_space_stream: &mut bool,
    ) {
        if !current.is_empty() {
            result.push(InlineElement {
                element: Inline::Str(current.clone()),
                slice: &slice[current_begin.unwrap()..=start],
            });
            *current = String::new();
            *current_begin = Some(start);
        }
        if let Some(x) = result.pop() {
            if x.element != Inline::Space {
                result.push(x);
            }
            result.push(InlineElement { element: Inline::SoftBreak, slice: &slice[start..=start] });
        }
        *is_space_stream = true;
    }

    fn handle_whitespace<'a>(
        slice: &'a str, result: &mut Vec<InlineElement<'a>>, current: &mut String,
        current_begin: &Option<usize>, char_iter: &mut Peekable<CharIndices<'a>>,
        is_space_stream: &mut bool, c: char, start: usize,
    ) {
        if c == ' ' {
            let mut two_spaces = false;
            while let Some(&(end, y)) = char_iter.peek() {
                if y == ' ' {
                    char_iter.next();
                    two_spaces = true;
                } else if y == '\n' && two_spaces {
                    result.push(InlineElement {
                        element: Inline::LineBreak,
                        slice: &slice[start..end],
                    });
                    break;
                } else {
                    break;
                }
            }
        }
        if !*is_space_stream {
            if !current.is_empty() {
                result.push(InlineElement {
                    element: Inline::Str(current.clone()),
                    slice: &slice[current_begin.unwrap()..start],
                });
            }
            result.push(InlineElement {
                element: Inline::Space,
                slice: &slice[start..start + c.len_utf8()],
            });
            *current = String::new();
            *is_space_stream = true;
        }
    }

    fn handle_regular_char(
        c: char, current: &mut String, current_begin: &mut Option<usize>, start: usize,
        is_prev_punctuation: &mut bool, is_space_stream: &mut bool,
    ) {
        *is_prev_punctuation = Self::ASCII_PUNCTUATION.contains(&c);
        *is_space_stream = false;
        if current_begin.is_none() {
            *current_begin = Some(start);
        }
        current.push(c);
    }

    // Assume `parse_hex_entity` and `parse_dec_entity` are defined elsewhere.


    #[allow(dead_code)]
    #[allow(clippy::too_many_lines)]
    fn parse_emph<'a>(
        base_string: &'a str, delimiter_stack: &mut [DelimiterStruct<'a>], stack_bottom: usize,
        result_vec: &mut [InlineElement<'a>],
    ) -> Vec<InlineElement<'a>> {
        let mut emph_vector: Vec<InlineElement> = Vec::new();
        // Nie moge przepisać na iterator bo iterator borrowuje wartość delimiter_stack
        for index in 0..delimiter_stack.len() {
            let mut delim = delimiter_stack[index].clone();
            match delim.typeof_delimiter {
                Potential::Opener | Potential::None => {},
                Potential::Both | Potential::Closer => {
                    let length = delim.count;
                    if index == 0 {
                        continue;
                    }
                    for j in (0..index).rev() {
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
                                if lower_bound < stack_bottom || upper_bound < stack_bottom {
                                    break;
                                }
                                result_vec[delimiter_stack[j].temp_vec.pop().unwrap()] =
                                    InlineElement { element: Inline::None, slice: "" };

                                let lower_res_index = delimiter_stack[j].temp_vec.pop().unwrap();
                                result_vec[lower_res_index] =
                                    InlineElement { element: Inline::None, slice: "" };
                                result_vec[delim.temp_vec.remove(0)] =
                                    InlineElement { element: Inline::None, slice: "" };
                                let upper_res_index = delim.temp_vec.remove(0);
                                result_vec[upper_res_index] =
                                    InlineElement { element: Inline::None, slice: "" };
                                let mut nested_inlines = Vec::new();
                                let mut is_last_str = false;
                                for x in lower_res_index..=upper_res_index {
                                    match &result_vec[x].element {
                                        Inline::Temp(c) => {
                                            if is_last_str {
                                                let temp = nested_inlines.pop().unwrap();
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
                                                let temp = nested_inlines.pop().unwrap();
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

                                if delim.delimiter_char == '~' {
                                    result_vec[lower_res_index] = InlineElement {
                                        element: Inline::Strikeout(nested_inlines.clone()),
                                        slice: &base_string[lower_bound..upper_bound],
                                    };
                                    emph_vector.push(InlineElement {
                                        element: Inline::Strikeout(nested_inlines),
                                        slice: &base_string[lower_bound..upper_bound],
                                    });
                                } else {
                                    result_vec[lower_res_index] = InlineElement {
                                        element: Inline::Strong(nested_inlines.clone()),
                                        slice: &base_string[lower_bound..upper_bound],
                                    };
                                    emph_vector.push(InlineElement {
                                        element: Inline::Strong(nested_inlines),
                                        slice: &base_string[lower_bound..upper_bound],
                                    });
                                }
                                let bottom_index =
                                    lower_bound + 2 - delimiter_stack[j].delim_slice.len();
                                delimiter_stack[j]
                                    .change_slice(&base_string[bottom_index..lower_bound]);
                                let top_index = upper_bound - 2 + delim.delim_slice.len();
                                delim.change_slice(&base_string[upper_bound..top_index]);
                                for delimiter in &mut delimiter_stack[j + 1..index] {
                                    delimiter.change_slice("");
                                    delimiter.typeof_delimiter = Potential::None;
                                    delimiter.delimiter_char = '-';
                                    delimiter.count = 0;
                                }
                            }
                            if !delim.delim_slice.is_empty()
                                && !delimiter_stack[j].delim_slice.is_empty()
                                && delim.delimiter_char == delimiter_stack[j].delimiter_char
                            {
                                let lower_res_index = delimiter_stack[j].temp_vec.pop().unwrap();
                                let upper_res_index = delim.temp_vec.remove(0);
                                result_vec[upper_res_index] =
                                    InlineElement { element: Inline::None, slice: "" };
                                result_vec[lower_res_index] =
                                    InlineElement { element: Inline::None, slice: "" };
                                let mut nested_inlines = Vec::new();
                                let mut is_last_str: bool = false;
                                for x in lower_res_index..=upper_res_index {
                                    let elem = &mut result_vec[x];
                                    match &elem.element {
                                        Inline::Temp(c) | Inline::Str(c) => {
                                            if is_last_str {
                                                if let Inline::Str(mut last) =
                                                    nested_inlines.pop().unwrap()
                                                {
                                                    last.push_str(c);
                                                    nested_inlines.push(Inline::Str(last));
                                                }
                                            } else {
                                                nested_inlines.push(Inline::Str(c.to_string()));
                                                is_last_str = true;
                                            }
                                            elem.element = Inline::None;
                                            elem.slice = "";
                                        },
                                        Inline::None => {
                                            elem.element = Inline::None;
                                            elem.slice = "";
                                        },
                                        _ => {
                                            nested_inlines.push(elem.element.clone());
                                            elem.element = Inline::None;
                                            elem.slice = "";
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
                                if delim.delimiter_char == '~' {
                                    result_vec[lower_res_index] = InlineElement {
                                        element: Inline::Strikeout(nested_inlines.clone()),
                                        slice: &base_string[lower_bound..upper_bound],
                                    };
                                    emph_vector.push(InlineElement {
                                        element: Inline::Strikeout(nested_inlines),
                                        slice: &base_string[lower_bound..upper_bound],
                                    });
                                } else {
                                    result_vec[lower_res_index] = InlineElement {
                                        element: Inline::Emph(nested_inlines.clone()),
                                        slice: &base_string[lower_bound..upper_bound],
                                    };
                                    emph_vector.push(InlineElement {
                                        element: Inline::Emph(nested_inlines),
                                        slice: &base_string[lower_bound..upper_bound],
                                    });
                                }
                                let bottom_index =
                                    lower_bound + 1 - delimiter_stack[j].delim_slice.len();
                                delimiter_stack[j]
                                    .change_slice(&base_string[bottom_index..lower_bound]);
                                let top_index = upper_bound - 1 + delim.delim_slice.len();
                                delim.change_slice(&base_string[upper_bound..top_index]);
                                for delimiter in delimiter_stack.iter_mut().take(index).skip(j + 1) {
                                    delimiter.change_slice("");
                                    delimiter.typeof_delimiter = Potential::None;
                                    delimiter.delimiter_char = '-';
                                    delimiter.count = 0;
                                }
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
