use crate::ast::Inline;

pub struct InlineParser;

impl InlineParser {
    pub fn parse(lines: Vec<String>) -> Vec<Inline> {
        //todo
        let mut result = Vec::new();
        for l in lines {
            let mut space = false;
            let mut current = String::new();
            for c in l.trim().chars() {
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
            result.push(Inline::SoftBreak);
        }
        result.pop();
        result
    }
}