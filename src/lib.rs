pub mod command;
pub mod config;
pub mod db;
pub mod download;
pub mod error;
pub mod files;
pub mod http;
pub mod mongo;
pub mod moving;
pub mod rename;
pub mod tmdb;
pub mod search;
pub mod openapi;
pub mod tests;

pub fn uppercase_words(data: &str) -> String {
    let mut result = String::new();
    let mut first = true;
    for value in data.chars() {
        if first {
            result.push(value.to_ascii_uppercase());
            first = false;
        } else {
            result.push(value);
            if value == ' ' {
                first = true;
            }
        }
    }
    result
}
