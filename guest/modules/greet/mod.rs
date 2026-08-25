#[allow(warnings)]
mod bindings;
use bindings::exports::example::hello::greeting::Guest;

struct Component;

impl Guest for Component {
    fn hello(name: String) {
        println!("hello {name}");
    }
}

bindings::export!(Component with_types_in bindings);