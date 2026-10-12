//! A development aid, not part of the core library: prints the view for a
//! file, to read while building.
//!
//!     cargo run -q -p tool_core --example dump -- data/sample-requests.csv '{"granularity":"quarter"}'
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&args[1]).expect("read file");
    let strings = std::fs::read_to_string("data/strings.json").expect("read strings");
    let settings = args.get(2).cloned().unwrap_or_else(|| "{}".into());
    println!("{}", tool_core::api::analyze(&text, &settings, &strings));
}
