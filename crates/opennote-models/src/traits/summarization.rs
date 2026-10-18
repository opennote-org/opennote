/// Implement this for structs that need a string summarization
pub trait Summarization {
    fn summarize(&self) -> String;
}

/// Generate a summarization for a vec of items implemented Summarization trait
pub fn summarize_all(items: Vec<impl Summarization>) -> String {
    let mut summarizations = String::new();

    for item in items {
        summarizations.push_str(&item.summarize());
    }

    summarizations
}
