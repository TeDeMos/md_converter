#![feature(unwrap_infallible, type_alias_impl_trait)]

use std::collections::HashMap;
use std::error::Error;
use std::fs;

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, Command};
use clap::builder::PossibleValuesParser;

use md_converter::ast::Pandoc;
use md_converter::ast::{Inline, Pandoc};
use md_converter::latex_writer::LatexWriter;
use md_converter::md_reader::inline_parser::InlineParser;
use md_converter::md_reader::MdReader;
use md_converter::native_reader::NativeReader;
use md_converter::native_writer::NativeWriter;
use md_converter::traits::{AstReader, AstWriter};
use md_converter::typst_writer::TypstWriter;

fn main() {
    let test = "a * foo bar*";
    let res = InlineParser::parse_lines(test);
    let a = Inline::Str("aaa".into());
    dbg!(a);
    // run()
    // let result = MdReader::read(
    //     "a\n:-"
    // )
    // .into_ok();
    // dbg!(result);
}

fn run() {
    let mut input_formats = ReaderMap::new();
    input_formats.add("gfm", || MdReader);
    input_formats.add("native", || NativeReader);
    let mut output_formats = WriterMap::new();
    output_formats.add("latex", LatexWriter::new);
    output_formats.add("typst", || TypstWriter);
    output_formats.add("native", || NativeWriter);
    let matches = Command::new("convert")
        .arg(
            Arg::new("from")
                .long("from")
                .short('f')
                .required(true)
                .action(ArgAction::Set)
                .value_parser(PossibleValuesParser::new(input_formats.keys()))
                .value_name("INPUT_FORMAT")
                .ignore_case(true),
        )
        .arg(
            Arg::new("to")
                .long("to")
                .short('t')
                .required(true)
                .action(ArgAction::Set)
                .value_parser(PossibleValuesParser::new(output_formats.keys()))
                .value_name("OUTPUT_FORMAT")
                .ignore_case(true),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .action(ArgAction::Set)
                .value_name("OUTPUT_FILE")
                .ignore_case(true),
        )
        .arg(Arg::new("file").required(true).index(1).action(ArgAction::Set).value_name("FILE"))
        .get_matches();
    let content = match fs::read_to_string(matches.get_one::<String>("file").unwrap()) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to read file:\n{}", e);
            return;
        },
    };
    let parsed = match input_formats.read(matches.get_one::<String>("from").unwrap(), &content) {
        Ok(p) => p,
        Err(e) => {
            println!("Failed to parse input format:\n{}", e);
            return;
        },
    };
    let result = match output_formats.write(matches.get_one::<String>("to").unwrap(), parsed) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to parse output format:\n{}", e);
            return;
        },
    };
    match matches.get_one::<String>("output") {
        Some(f) => match fs::write(f, result) {
            Ok(_) => println!("Saved result to: {}", f),
            Err(e) => println!("Failed to save file:\n{}", e),
        },
        None => println!("{}", result),
    }
}

pub type Reader = Box<dyn Fn(&str) -> Result<Pandoc, Box<dyn Error>>>;

pub struct ReaderMap(HashMap<&'static str, Reader>);

impl ReaderMap {
    pub fn new() -> Self { Self(HashMap::new()) }

    pub fn add<T, E>(&mut self, name: &'static str, reader_creator: fn() -> T)
    where
        T: AstReader<ReadError = E> + 'static,
        E: Error + 'static,
    {
        self.0.insert(
            name,
            Box::new(move |s| match reader_creator().read(s) {
                Ok(p) => Ok(p),
                Err(e) => Err(Box::new(e)),
            }),
        );
    }

    pub fn keys(&self) -> impl Iterator<Item = &&'static str> { self.0.keys() }

    pub fn read(&self, name: &str, source: &str) -> Result<Pandoc, Box<dyn Error>> {
        self.0.get(name).unwrap()(source)
    }
}

pub type Writer = Box<dyn Fn(Pandoc) -> Result<String, Box<dyn Error>>>;

pub struct WriterMap(HashMap<&'static str, Writer>);

impl WriterMap {
    pub fn new() -> Self { Self(HashMap::new()) }

    pub fn add<T, E>(&mut self, name: &'static str, writer_creator: fn() -> T)
    where
        T: AstWriter<WriteError = E> + 'static,
        E: Error + 'static,
    {
        self.0.insert(
            name,
            Box::new(move |p| match writer_creator().write(p) {
                Ok(s) => Ok(s),
                Err(e) => Err(Box::new(e)),
            }),
        );
    }

    pub fn keys(&self) -> impl Iterator<Item = &&'static str> { self.0.keys() }

    pub fn write(&self, name: &str, pandoc: Pandoc) -> Result<String, Box<dyn Error>> {
        self.0.get(name).unwrap()(pandoc)
    }
}
