#![recursion_limit="80"]

pub extern crate xmlparser;
extern crate codegen;
extern crate heck;
extern crate num_traits;
extern crate bigdecimal;

#[macro_use] pub mod macros;
pub mod xml_utils;
pub mod names;
pub mod support;
pub mod primitives;
pub mod bigfloat;

pub mod parser;
pub mod processor;
pub mod parser_generator;

#[cfg(test)]
mod test_parser;
#[cfg(test)]
mod test_parser_schema;

use support::{ParseXml, InnerStream, ParseContext};

pub use processor::Processor;
pub use parser_generator::ParserGenerator;

pub fn parse_xsd(xsd: &str) -> parser::xs::Schema {
    let tokenizer = xmlparser::Tokenizer::from(xsd);
    let mut stream = Box::new(InnerStream::new(tokenizer));
    parser::xs::Schema::parse_xml(&mut stream, &mut ParseContext::default(), &()).unwrap()
}
