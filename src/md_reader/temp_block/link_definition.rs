use std::collections::HashMap;
use crate::md_reader::temp_block::iters::SkipIndent;
use crate::md_reader::temp_block::{LineResult};

#[derive(Debug)]
pub struct LinkDefinition {
    pub stripped: String,
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug)]
pub struct Link {
    pub url: String,
    pub title: Option<String>,
}

impl LinkDefinition {
    pub fn next(&mut self, line: SkipIndent) -> LineResult {
        todo!()
    }

    pub fn next_blank(&mut self) -> (LineResult, bool) {
        todo!()
    }
}

#[derive(Debug, Default)]
pub struct Links(HashMap<String, Link>);

impl Links {
    pub fn new() -> Self { Self(HashMap::new()) }

    pub fn add(&mut self, stripped: String, link: Link) {
        self.0.entry(stripped).or_insert(link);
    }

    pub fn add_from(&mut self, link: LinkDefinition) {
        self.0.entry(link.stripped).or_insert_with(|| Link { url: link.url, title: link.title });
    }

    pub fn extend(&mut self, new: Self) {
        for (k, v) in new.0 {
            self.add(k, v);
        }
    }
    
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

