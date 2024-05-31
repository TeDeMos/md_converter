use std::iter::Peekable;
use std::str::CharIndices;
use std::string::String;

use crate::ast::{attr_empty, Inline};

pub struct InlineParser {}

enum HtmlEntityState {
    Hex,
    Dec,
    NoState,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Potential {
    Opener,
    Closer,
    Both,
}

#[allow(dead_code)]
#[derive(Clone)]
struct DelimiterStruct<'a> {
    count: usize,
    is_strong: bool,
    delimiter_char: char,
    delim_slice: &'a str,
    typeof_delimiter: Potential,
}

#[allow(dead_code)]
impl<'a> DelimiterStruct<'a> {
    fn print_debug(&self) {
        println!(
            "DelimiterStruct {{ count: {}, is_strong: {}, delimiter_char: '{}', delim_slice: \"{}\", typeof_delimiter: {:?} }}",
            self.count,
            self.is_strong,
            self.delimiter_char,
            self.delim_slice,
            self.typeof_delimiter
        );
    }
    fn change_slice(&mut self, new_slice: &'a str) {
        self.delim_slice = new_slice;
    }
}

#[allow(dead_code)]
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
                            result.push(BacktickString {
                                backtick_length: e - s,
                                start_index: s,
                            });
                            break;
                        }
                        None => {
                            result.push(BacktickString {
                                backtick_length: paragraph.len() - s,
                                start_index: s,
                            });
                            break;
                        }
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
        paragraph: &str,
        backtick_vec: Vec<BacktickString>,
    ) -> Vec<SliceVariant> {
        let mut open_iter = backtick_vec.iter();
        let mut prev_index = 0;
        let mut result: Vec<SliceVariant> = Vec::new();
        loop {
            if let Some(c) = open_iter.next() {
                if (c.start_index != 0) {
                    result.push(SliceVariant::InlineSlice(
                        &paragraph[prev_index..c.start_index],
                    ));
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
                    result.push(SliceVariant::InlineSlice(
                        &paragraph[prev_index..paragraph.len()],
                    ));
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

    pub fn parse_lines(paragraph: &str) -> Vec<Inline> {
        let inlines_and_code = Self::parse_code_spans(paragraph);
        let mut last_opener_star: [Option<usize>; 3] = [None; 3];
        let mut last_opener_floor: [Option<usize>; 3] = [None; 3];
        let mut result: Vec<InlineElement> = Vec::new();
        let mut delimiter_stack: Vec<DelimiterStruct> = Vec::new();
        let mut iter = inlines_and_code.iter();

        loop {
            match iter.next() {
                Some(&SliceVariant::CodeSlice(x)) => {
                    //Check if emphasis open then prepare the CODE inline
                    result.push(Self::parse_code_slice(x));
                    //println!("Code {x}");
                }
                Some(&SliceVariant::InlineSlice(x)) => {
                    delimiter_stack.append(&mut Self::parse_inline_slice(
                        x,
                        &mut result,
                        &mut last_opener_star,
                        &mut last_opener_floor,
                    ));
                    //println!("Inline {x}");
                }
                None => break,
            }
        }
        let mut true_result: Vec<Inline> = Vec::new();
        // for x in result {
        //     true_result.push(x.element);
        //     println!("{}", x.slice);
        // }

        let mut in_vec = parse_emph(
            paragraph,
            &mut delimiter_stack,
            &last_opener_star,
            &last_opener_floor,
        );

        for x in in_vec {
            print!("{}\n", x.slice);
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
            InlineElement {
                element: Inline::Code(attr_empty(), result.parse().unwrap()),
                slice,
            }
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
                }
                Some((_, c @ ';')) => {
                    if !current_bonus.is_empty() {
                        let entity_value = u32::from_str_radix(&current_bonus, 16);
                        return match entity_value {
                            Ok(x) => {
                                copy_iter.next();
                                (
                                    StringOrChar::HTMLChar(char::from_u32(x).unwrap()),
                                    copy_iter,
                                )
                            }
                            Err(_) => (
                                StringOrChar::HTMLChar(char::from_u32(0xfffd).unwrap()),
                                copy_iter,
                            ),
                        };
                    }
                }
                Some((_, c)) => return (StringOrChar::NoHTMLString(current_bonus), begin_iter),
                None => {}
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
                }
                Some((_, c @ ';')) => {
                    if !current_bonus.is_empty() {
                        let entity_value = u32::from_str_radix(&*current_bonus, 10);
                        return match entity_value {
                            Ok(x) => {
                                copy_iter.next();
                                (
                                    StringOrChar::HTMLChar(char::from_u32(x).unwrap()),
                                    copy_iter,
                                )
                            }
                            Err(_) => (
                                StringOrChar::HTMLChar(char::from_u32(0xfffd).unwrap()),
                                copy_iter,
                            ),
                        };
                    }
                }
                Some((_, c)) => return (StringOrChar::NoHTMLString(current_bonus), begin_iter),
                None => return (StringOrChar::NoHTMLString(current_bonus), begin_iter),
            }
            length += 1;
        }
        return (StringOrChar::NoHTMLString(current_bonus), begin_iter);
    }
    #[allow(clippy::too_many_lines)]
    fn parse_inline_slice<'a>(
        slice: &'a str,
        result: &mut Vec<InlineElement<'a>>,
        last_opener_star: &mut [Option<usize>; 3],
        last_opener_floor: &mut [Option<usize>; 3],
    ) -> Vec<DelimiterStruct<'a>> {
        const ASCII_PUNCTUATION: [char; 26] = [
            '!', '"', '#', '%', '&', '\'', '(', ')', '*', ',', '.', '/', ':', ';', '?', '@', '[',
            '\\', ']', '^', '_', '`', '{', '}', '|', '~',
        ];
        const UNICODE_WHITESPACE: [char; 25] = [
            '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0020}', '\u{0085}',
            '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}',
            '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}',
            '\u{2029}', '\u{202F}', '\u{205F}', '\u{3000}',
        ];
        let mut delimiter_stack: Vec<DelimiterStruct> = Vec::new();
        let mut is_space_stream: bool = false;
        let mut current: String = String::new();
        let mut html_current: String = String::new();
        let mut char_iter = slice.char_indices().peekable();
        let mut is_beginning: bool = true;
        let mut current_begin: i32 = -1;
        let mut is_prev_punctuation: bool = false;
        loop {
            match char_iter.next() {
                Some((start, c @ ('*' | '_'))) => {
                    let mut is_left_run = false;
                    let mut is_right_run = false;
                    let mut length = 1;
                    let mut is_strong = true;
                    let mut followed_by_punctuation = false;
                    let mut followed_by_whitespace = false;
                    let mut end_slice: usize = 0;
                    if !current.is_empty() {
                        //reset current after push
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
                                last_opener_floor[end_slice - start % 3] =
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
                    if is_left_run && is_right_run {
                        delimiter_stack.push(DelimiterStruct {
                            count: length,
                            is_strong,
                            delimiter_char: c,
                            delim_slice: &slice[start..end_slice],
                            typeof_delimiter: Potential::Both,
                        });
                    } else if is_left_run {
                        delimiter_stack.push(DelimiterStruct {
                            count: length,
                            is_strong,
                            delimiter_char: c,
                            delim_slice: &slice[start..end_slice],
                            typeof_delimiter: Potential::Opener,
                        });
                    } else {
                        delimiter_stack.push(DelimiterStruct {
                            count: length,
                            is_strong,
                            delimiter_char: c,
                            delim_slice: &slice[start..end_slice],
                            typeof_delimiter: Potential::Closer,
                        });
                    }
                }
                Some((_, '\\')) => {
                    is_space_stream = false;
                    if let Some((_, peek_char)) = char_iter.next() {
                        if !ASCII_PUNCTUATION.contains(&peek_char) {
                            current.push('\\');
                            is_prev_punctuation = true;
                        }
                        current.push(peek_char);
                    }
                }
                Some((index, '&')) => {
                    is_prev_punctuation = false;
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
                                        }
                                        StringOrChar::HTMLChar(c) => {
                                            current.push(c);
                                        }
                                    }
                                }
                                Some((_, _)) => {
                                    Self::parse_dec_entity(char_iter.clone());
                                    let mut parse_result: StringOrChar;
                                    (parse_result, char_iter) =
                                        Self::parse_dec_entity(char_iter.clone());
                                    match parse_result {
                                        StringOrChar::NoHTMLString(_) => {
                                            current.push_str(&html_current);
                                        }
                                        StringOrChar::HTMLChar(c) => {
                                            if current_begin == -1 {
                                                current_begin = begin_index as i32;
                                            }
                                            current.push(c);
                                        }
                                    }
                                }
                                None => {
                                    current.push_str(&html_current);
                                }
                            }
                            html_current = String::new();
                        }
                        Some((_, _)) => {}
                        None => {}
                    }
                }
                Some((index, c)) if UNICODE_WHITESPACE.contains(&c) => {
                    is_prev_punctuation = false;
                    if !is_space_stream {
                        if !current.is_empty() {
                            result.push(InlineElement {
                                element: Inline::Str(current),
                                slice: &slice[current_begin as usize..index],
                            });
                        }
                        result.push(InlineElement {
                            element: Inline::Space,
                            slice: &slice[index..index + 1],
                        });
                        current = String::new();
                        is_space_stream = true;
                    }
                }
                Some((index, c)) => {
                    if is_space_stream {
                        result.push(InlineElement {
                            element: Inline::Space,
                            slice: &slice[index..index + 1],
                        });
                    }
                    if (ASCII_PUNCTUATION.contains(&c)) {
                        is_prev_punctuation = true;
                    }
                    is_space_stream = false;
                    if current_begin == -1 {
                        current_begin = index as i32;
                    }
                    current.push(c);
                }
                None => {
                    if current.len() != 0 {
                        result.push(InlineElement {
                            element: Inline::Str(current),
                            slice: &slice[current_begin as usize..slice.len()],
                        });
                    }
                    break;
                }
            }
            is_beginning = false;
        }
        return delimiter_stack;
    }

    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn temp_method(lines: &[String]) -> Vec<Inline> {
        //todo
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
                //Space remover
                match char_iter.peek() {
                    Some((_, '\t' | ' ')) => {}
                    Some(_) => space = false,
                    None => {}
                }

                //HtmlEntityValue check
                if matches!(
                    html_entity_state,
                    HtmlEntityState::Dec | HtmlEntityState::Hex
                ) {
                    match char_iter.next() {
                        Some((end_index, ';')) => {
                            length = 0;
                            let entity_value = u32::from_str_radix(
                                &l.trim()[start_slice_index..end_index],
                                html_entity_state.get_base(),
                            );
                            match entity_value {
                                Ok(value) => current.push(char::from_u32(value).unwrap()),
                                Err(_) => current.push(char::from_u32(0xFFFD).unwrap()),
                            }
                            html_entity_state = HtmlEntityState::NoState;
                        }
                        Some((_, _c @ ('0'..='9')))
                            if length < html_entity_state.get_entity_max_length() =>
                        {
                            length += 1;
                        }
                        Some((_, ('a'..='f' | 'A'..='F')))
                            if html_entity_state.get_base() == 16 =>
                        {
                            if length < html_entity_state.get_entity_max_length() {
                                length += 1;
                            }
                        }
                        //This could be written better I think
                        Some((_, ('x' | 'X'))) if html_entity_state.get_base() == 16 => {
                            continue;
                        }

                        Some((end_index, _)) => {
                            html_entity_state = HtmlEntityState::NoState;
                            length = 0;
                            current.push_str(&char_line[start_slice_index..end_index]);
                        }
                        None => {}
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
                    }
                    Some((_, ' ' | '\t')) if !space => {
                        result.push(Inline::Str(current));
                        current = String::new();
                        result.push(Inline::Space);
                        space = true;
                    }
                    Some((_, ' ' | '\t')) if space => {}
                    Some((_, '\\')) => {
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
                        }
                    }
                    Some((pos, '&')) => {
                        start_slice_index = pos;
                        match char_iter.peek() {
                            Some((_, '#')) => {
                                char_iter.next();
                                match char_iter.peek() {
                                    Some((index, ('X' | 'x'))) => {
                                        html_entity_state = HtmlEntityState::Hex;
                                        start_slice_index = *index + 1;
                                    }
                                    Some((index, _)) => {
                                        html_entity_state = HtmlEntityState::Dec;
                                        start_slice_index = *index;
                                    }
                                    None => {}
                                }
                            }
                            Some(_) => {
                                todo!()
                            }
                            None => {}
                        }
                    }
                    Some((_, c)) => {
                        current.push(c);
                    }
                    None => {
                        if !current.is_empty() {
                            result.push(Inline::Str(current));
                            current = String::new();
                        }
                        result.push(Inline::SoftBreak);
                        break;
                    }
                }
            }
        }
        result.pop();
        result
    }

    #[must_use]
    pub fn parse_vector(vector: &[String]) -> Vec<Inline> {
        Self::parse_lines(&vector.join(""))
    }

    pub fn parse_line(line: String) -> Vec<Inline> {
        Self::parse_lines(&line)
    }

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

#[allow(dead_code)]
fn parse_emph<'a>(
    base_string: &'a str,
    delimiter_stack: &mut Vec<DelimiterStruct<'a>>,
    last_opener_star: &[Option<usize>; 3],
    last_opener_floor: &[Option<usize>; 3],
) -> Vec<InlineElement<'a>> {
    let mut emph_vector: Vec<InlineElement> = Vec::new();
    //Nie moge przepisać na iterator bo iterator borrowuje wartość delimiter_stack
    for index in 0..delimiter_stack.len() {
        let mut delim = delimiter_stack[index].clone();
        match delim.typeof_delimiter {
            Potential::Opener => {}
            Potential::Both | Potential::Closer => {
                let length = delim.delim_slice.len();
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
                for j in (0..=index - 1).rev() {
                    if !matches!(delimiter_stack[j].typeof_delimiter, Potential::Closer)
                        && ((matches!(delimiter_stack[j].typeof_delimiter, Potential::Both)
                            || matches!(delim.typeof_delimiter, Potential::Both))
                            && (delimiter_stack[j].delim_slice.len() + length) % 3 != 0
                            || (length % 3 == 0 && delimiter_stack[j].delim_slice.len() % 3 == 0))
                        || (matches!(delimiter_stack[j].typeof_delimiter, Potential::Opener)
                            && matches!(delim.typeof_delimiter, Potential::Closer))
                            && delimiter_stack[j].delimiter_char == delim.delimiter_char
                    {
                        while delim.delim_slice.len() >= 2
                            && delimiter_stack[j].delim_slice.len() >= 2
                        {
                            let lower_bound = delimiter_stack[j].delim_slice.as_ptr() as usize
                                - base_string.as_ptr() as usize
                                + delimiter_stack[j].delim_slice.len()
                                - 2;
                            let upper_bound = delim.delim_slice.as_ptr() as usize
                                - base_string.as_ptr() as usize
                                + 2;
                            emph_vector.push(InlineElement {
                                element: Inline::Strong(vec![]),
                                slice: &base_string[lower_bound..upper_bound],
                            });
                            let bottom_index =
                                lower_bound + 2 - delimiter_stack[j].delim_slice.len();
                            delimiter_stack[j]
                                .change_slice(&base_string[bottom_index..lower_bound]);
                            let top_index = upper_bound - 2 + delim.delim_slice.len();
                            delim.change_slice(&base_string[upper_bound..top_index]);
                        }
                        if !delim.delim_slice.is_empty()
                            && !delimiter_stack[j].delim_slice.is_empty()
                        {
                            let lower_bound = delimiter_stack[j].delim_slice.as_ptr() as usize
                                - base_string.as_ptr() as usize
                                + delimiter_stack[j].delim_slice.len()
                                - 1;
                            let upper_bound = delim.delim_slice.as_ptr() as usize
                                - base_string.as_ptr() as usize
                                + 1;
                            emph_vector.push(InlineElement {
                                element: Inline::Emph(vec![]),
                                slice: &base_string[lower_bound..upper_bound],
                            });
                            let bottom_index =
                                lower_bound + 1 - delimiter_stack[j].delim_slice.len();
                            delimiter_stack[j]
                                .change_slice(&base_string[bottom_index..lower_bound]);
                            let top_index = upper_bound - 1 + delim.delim_slice.len();
                            delim.change_slice(&base_string[upper_bound..top_index]);
                        }

                        if matches!(delimiter_stack[j].typeof_delimiter, Potential::Opener) {
                        } else if matches!(delimiter_stack[j].typeof_delimiter, Potential::Both) {
                        }
                    }
                }
                delimiter_stack[index] = delim;
            }
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
