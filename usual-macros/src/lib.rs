#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;

mod usual;

#[proc_macro_attribute]
pub fn petelib(attr: TokenStream, item: TokenStream) -> TokenStream {
    usual::petelib(attr, item)
}
