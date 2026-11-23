pub mod ast;
pub mod comment;
pub mod html;
pub mod parser;

pub use ast::{Alignment, Block, Document, FootnoteDef, Inline, ListItem};
pub use html::render_document;
pub use parser::parse;

pub fn to_html(input: &str) -> String {
    let doc = parse(input);
    render_document(&doc)
}
