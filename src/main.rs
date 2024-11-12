#![allow(dead_code, unreachable_code)]

use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

mod cli;

fn main() {
    let spinner_chars = ['⠋', '⠙', '⠸', '⠴', '⠦', '⠇'];
    let mut index = 0;

    // Simulate a long-running task
    loop {
        print!("\r{}", spinner_chars[index]);
        io::stdout().flush().unwrap();

        index = (index + 1) % spinner_chars.len();
        thread::sleep(Duration::from_millis(100)); // Adjust the speed of the spinner
    }

    println!("\rDone!");
}
