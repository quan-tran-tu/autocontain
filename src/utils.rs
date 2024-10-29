use std::io::{self};

pub fn user_exit() {
    println!("Press Enter to exit.");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to exit.");
}
