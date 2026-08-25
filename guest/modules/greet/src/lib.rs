mod handlers;

use shared::bindings::exports::app::greeting::greeting::Guest;

struct Component;

impl Guest for Component {
    fn hello(name: String) { handlers::greet(name); }
}

shared::bindings::export!(Component with_types_in shared::bindings);