extern crate smithy;

use smithy::Smithy;

fn main() {
    match Smithy::builder("input", "output").build() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    };
}
