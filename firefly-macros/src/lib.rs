use proc_macro::TokenStream;

mod firefly;
mod firefly_prelude;
mod firefly_route;

#[proc_macro_attribute]
pub fn firefly(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::firefly::firefly(attr, item)
}

#[proc_macro]
pub fn firefly_prelude(attr: TokenStream) -> TokenStream {
    crate::firefly_prelude::prelude(attr)
}

#[proc_macro_attribute]
pub fn firefly_route(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::firefly_route::json_request(attr, item)
}
