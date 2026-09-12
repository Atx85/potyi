mod greetings {
    pub fn greet(name: &str) -> String {
        format!("Hello, {name}!")
    }
}

fn main() {
    // Deliberately missing: use std::collections::HashMap;
    let mut scores = HashMap::new();
    scores.insert("Alice", 42);
    scores.insert("Bob", 27);

    for (name, score) in &scores {
        println!("{} Score: {score}", greetings::greet(name));
    }
}