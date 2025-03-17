mod default_storage;
use crate::default_storage::storage;

mod default_networking;
use crate::default_networking::networking;

#[cfg(test)]
mod tests;

mod errors;
mod helpers;
mod types;

fn main() {
    println!("Hello, world!");
}
