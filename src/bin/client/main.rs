use std::io;

fn main() {
    println!("Started client.");

    println!("Enter Username:");
    let mut username = String::new();
    io::stdin().read_line(&mut username).unwrap();

    println!("Enter Password:");
    let mut password = String::new();
    io::stdin().read_line(&mut password).unwrap();
}