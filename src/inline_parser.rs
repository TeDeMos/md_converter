use crate::ast::Inline;

pub struct InlineParser;

impl InlineParser {
    pub fn parse_lines(lines: &[String]) -> Vec<Inline> {
        //todo
        let mut result = Vec::new();
        for l in lines {
            Self::parse_one_line(l, &mut result);
            result.push(Inline::SoftBreak);
        }
        result.pop();
        result
    }
    
    pub fn parse_line(line: &str) -> Vec<Inline> {
        let mut result = Vec::new();
        Self::parse_one_line(line, &mut result);
        result
    }

    fn parse_one_line(line: &str, result: &mut Vec<Inline>) {
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
            } else {
                if matches!(c, ' ' | '\t') {
                    result.push(Inline::Str(current));
                    current = String::new();
                    space = true;
                } else {
                    current.push(c);
                }
            }
        }
        if !current.is_empty() {
            result.push(Inline::Str(current));
        }
    }
}