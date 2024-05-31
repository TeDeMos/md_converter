use std::collections::HashMap;

#[derive(Debug)]
pub struct Link {
    pub url: String,
    pub title: Option<String>,
}

impl Link {
    fn new(url: &str, title: Option<&str>) -> Self {
        Self {
            url: url.to_owned(),
            title: title.map(str::to_owned),
        }
    }
}

#[derive(Debug, Default)]
pub struct Links(HashMap<String, Link>);

impl Links {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

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
                }
            }
        }
        result
    }

    pub fn add_new(&mut self, unstripped: &str, destination: &str, title: Option<&str>) {
        self.0
            .entry(Self::strip(unstripped))
            .or_insert_with(|| Link::new(destination, title));
    }

    pub fn get(&self, stripped: &str) -> Option<&Link> {
        self.0.get(stripped)
    }
}
