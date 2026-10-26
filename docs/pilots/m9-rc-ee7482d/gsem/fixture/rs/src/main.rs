fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

fn main() {
    let world = greet("world");
    let mars = greet("mars");
    println!("{} {}", world, mars);
}