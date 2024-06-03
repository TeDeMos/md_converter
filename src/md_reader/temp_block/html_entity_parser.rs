use std::collections::HashMap;
use std::fs;

use serde::Deserialize;
use serde_json::Result;

#[derive(Deserialize)]
struct Entity {
    characters: String,
}

fn create_hashmap() -> Result<()> {
    // Read the JSON file
    let file_content = fs::read_to_string("entities.json").expect("Unable to read file");

    // Parse the JSON content
    let entities: HashMap<String, Entity> = serde_json::from_str(&file_content)?;

    // Create the HashMap to store the entities and their corresponding characters
    let mut html_entities: HashMap<&str, &str> = HashMap::new();

    for (entity, details) in &entities {
        html_entities.insert(entity, &details.characters);
    }

    // Example usage
    for (entity, character) in &html_entities {
        println!("{}: {}", entity, character);
    }

    Ok(())
}
