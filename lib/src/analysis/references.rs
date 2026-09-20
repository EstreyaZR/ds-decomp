#![allow(dead_code, unused)]
use std::{fmt::Display, path::Path, rc::Rc, str::FromStr, vec::IntoIter};

use snafu::{Whatever, prelude::*};

use crate::{
    config::{ParseContext, section::SectionKind},
    util::{io::read_to_string, parse::parse_u16},
};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum GraphNodeStatus {
    #[default]
    PreInit,
    ScanningChildren,
    Grouping,
    Finished,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionType {
    #[default]
    Arm,
    Thumb,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum GraphNodeType {
    Function(FunctionType),
    ThumbFunction,
    Local,
    // Label,
    #[default]
    Data,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
struct GraphNode {
    name: String,
    status: GraphNodeStatus,
    node_type: GraphNodeType,
    children: Vec<String>,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Graph {
    nodes: Vec<GraphNode>,
}

impl GraphNode {
    pub fn new(name: String) -> Self {
        Self { name, ..Default::default() }
    }

    pub fn set_status(&mut self, new_status: GraphNodeStatus) {
        todo!()
    }
}

/// Struct that is built by every symbol / label declaration with form `"NAME: Option<WORD>"`
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct GraphEntry {
    name: Option<String>,
    label: bool,
    /// None: Symbol is a Label to jump to, used in goto, etc...
    /// Some: Contains a Word Directive
    ///
    /// Example of Behaviour:
    /// ```
    /// ".L_0x1234:" => None,
    /// ".L_0x6767: .word data_importantData" => Some(data_importantData),
    /// ```
    byte: Option<Vec<u16>>,
    byte_as_str: Option<String>,
    // TODO: Add Kind of Word; e.g. Numerical or Symbol
    word: Option<String>,
}

impl GraphEntry {
    /// Assumes not to fail from input
    pub fn from_str(source: &str) -> Result<Self, Whatever> {
        let source: Vec<&str> = source.split_whitespace().collect();

        if source[0].contains(".L_") {
            let label = true;
            let mut word = None;
            let name = Some(source[0].strip_suffix(":").unwrap().to_string());
            if source.len() > 1 {
                word = Some(source[2].to_string());
            }
            return Ok(Self { name, label, word, byte: None, byte_as_str: None });
        }

        if source[0].contains(".byte") {
            let byte: Vec<u16> = match source.len() {
                1 => vec![parse_u16(source[1]).unwrap()],
                2.. => source[1..]
                    .iter()
                    .map(|x| {
                        if x.contains(",") {
                            parse_u16(x.strip_suffix(",").unwrap()).unwrap()
                        } else {
                            parse_u16(x).unwrap()
                        }
                    })
                    .collect(),
                0 => unreachable!(),
            };

            let byte_string = String::from_utf16(&byte.clone().as_slice()).unwrap();
            Ok(Self {
                name: None,
                label: false,
                byte: Some(byte),
                byte_as_str: Some(byte_string),
                word: None,
            })
        } else if source[0].contains(".word") {
            let name = Some(source[1].to_string());
            let label = false;
            let word: Option<String> = match label {
                true => {
                    if source[1].contains(".word") {
                        Some(source[2].into())
                    } else {
                        None
                    }
                }
                false => {
                    if source[0].contains(".word") {
                        Some(source[1].into())
                    } else {
                        None
                    }
                }
            };
            Ok(Self { name, label, word, byte: None, byte_as_str: None })
        } else {
            let name = Some(source[0].strip_suffix(":").unwrap().to_string());
            let label = source[0].contains(".L_");
            let word: Option<String> = match label {
                true => {
                    if source[1].contains(".word") {
                        Some(source[2].into())
                    } else {
                        None
                    }
                }
                false => {
                    if source[0].contains(".word") {
                        Some(source[1].into())
                    } else {
                        None
                    }
                }
            };
            Ok(Self { name, label, word, byte: None, byte_as_str: None })
        }
    }
}

impl Display for GraphEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::new();

        if let Some(name) = &self.name {
            output.push_str(format!("{}\tLabel:{}\n", name, self.label).as_str());
        } else {
            output.push_str(format!("N/A\tLabel: {}\n", self.label).as_str());
        }
        if let Some(byte) = &self.byte {
            output.push_str(format!("Data:\t\t{:x?}\n", byte).as_str());
        }
        if let Some(byte) = &self.byte_as_str {
            output.push_str(format!("AsString:\t\t{}\n", byte).as_str());
        }

        output.fmt(f)
    }
}

/// Simple Vector Collection for [GraphEntry]
#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
struct GraphEntries(Vec<GraphEntry>);

impl GraphEntries {
    fn add(&mut self, x: GraphEntry) {
        self.0.push(x)
    }
}

impl IntoIterator for GraphEntries {
    type IntoIter = IntoIter<Self::Item>;
    type Item = GraphEntry;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

struct GraphParser();

impl GraphParser {
    fn entries<P: AsRef<Path>>(file: P) -> Result<GraphEntries, Whatever> {
        let file = file.as_ref();
        let mut buffer = read_to_string(file).unwrap();
        let mut lines = buffer.lines();

        // filter everything except definitions, words and bytes
        let lines: Vec<_> = lines
            .filter(|line| line.contains(":") | line.contains(".word") | line.contains(".byte"))
            .collect();

        let mut defs = GraphEntries::default();
        for def_line in lines {
            defs.add(GraphEntry::from_str(def_line).unwrap());
        }

        Ok(defs)
    }
}

impl Graph {
    fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn from_file<P: AsRef<Path>>(file: P) /*-> Result<Self, Whatever>*/
    {
        let file = file.as_ref();
        let mut graph = Graph::new();

        let mut buffer = read_to_string(file).unwrap();
        let mut lines = buffer.lines();

        let entries = GraphParser::entries(file).unwrap();
        for i in entries {
            println!("{}", i);
        }
    }
}
