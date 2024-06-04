use std::{fs, io};
use std::io::Read;

use clap::{Arg, ArgAction, Command};
use clap::builder::PossibleValuesParser;

use md_converter::latex_writer::LatexWriter;
use md_converter::maps::{ReaderMap, WriterMap};
use md_converter::md_reader::MdReader;
use md_converter::native_reader::NativeReader;
use md_converter::native_writer::NativeWriter;
use md_converter::typst_writer::TypstWriter;

fn main() {
    // let test =
    // "\\!\\\"\\#\\$\\%\\&\\\'\\(\\)\\*\\+\\,\\-\\.\\/\\:\\;\\<\\=\\>\\?\\@\\[\\]\\^\\_\\\
    //             `\\{\\|\\}\\~";
    // for x in InlineParser::parse_lines(test) {
    //     print!("{:?}", x);
    // }
    run()
}

fn run() {
    let mut input_formats = ReaderMap::new();
    input_formats.add("gfm", || MdReader);
    input_formats.add("native", || NativeReader);
    let mut output_formats = WriterMap::new();
    output_formats.add("latex", LatexWriter::new);
    output_formats.add("typst", TypstWriter::new);
    output_formats.add("native", || NativeWriter);
    let matches = Command::new("convert")
        .version("1.0")
        .author("Tymoteusz Malec, Jakub Szweda")
        .about(
            "Converts files from one format to another or reads from stdin if no filename is \
             provided",
        )
        .arg(
            Arg::new("from")
                .long("from")
                .short('f')
                .help("Format of the input")
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
                .help("Target format to convert to")
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
        .arg(Arg::new("file").index(1).action(ArgAction::Set).value_name("FILE"))
        .get_matches();
    let content = match matches.get_one::<String>("file") {
        Some(f) => match fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                println!("Failed to read file:\n{}", e);
                return;
            },
        },
        None => {
            let mut s = String::new();
            match io::stdin().read_to_string(&mut s) {
                Ok(_) => s,
                Err(e) => {
                    println!("Failed to read input from stdin:\n{}", e);
                    return;
                },
            }
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
