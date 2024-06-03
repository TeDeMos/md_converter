use std::collections::HashMap;

/// Represents a link in a GitHub Flavoured Markdown document
#[derive(Debug)]
pub struct Link {
    /// Url this link
    pub url: String,
    /// Optional title of this link
    pub title: Option<String>,
}

impl Link {
    /// Creates a new link by copying slices
    fn new(url: &str, title: Option<&str>) -> Self {
        Self { url: url.to_owned(), title: title.map(str::to_owned) }
    }
}

/// Represents links found in the document
#[derive(Debug, Default)]
pub struct Links(HashMap<String, Link>);

impl Links {
    /// Creates a new empty collection of links
    pub fn new() -> Self { Self(HashMap::new()) }

    /// Strips a key for matching or inserting
    pub fn strip(key: &str) -> String {
        let mut space = false;
        let mut result = String::new();
        for c in key.trim().chars() {
            match c {
                ' ' | '\t' | '\n' => space = true,
                c => {
                    if space {
                        space = false;
                        result.push(' ');
                    }
                    for c in c.to_lowercase() {
                        result.push(c);
                    }
                },
            }
        }
        result
    }

    /// Adds new link if not already present
    pub fn add_new(&mut self, unstripped: &str, destination: &str, title: Option<&str>) {
        self.0.entry(Self::strip(unstripped)).or_insert_with(|| Link::new(destination, title));
    }

    /// Gets link from collection if present
    pub fn get(&self, stripped: &str) -> Option<&Link> { self.0.get(stripped) }
    
    /// Returns amount of links in the collection
    pub fn len(&self) -> usize { self.0.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip() {
        assert_eq!(Links::strip("    before").as_str(), "before");
        assert_eq!(Links::strip("after      ").as_str(), "after");
        assert_eq!(Links::strip(" \n both \n ").as_str(), "both");
        assert_eq!(Links::strip("  internal   \n   spaces \n ").as_str(), "internal spaces");
    }
}