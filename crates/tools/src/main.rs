mod cli;
mod utils;
use tracing_subscriber;

fn main() {
    println!("DOTVM!");
}

#[test]
fn test_main() {
    main();
    assert!(true);
}
