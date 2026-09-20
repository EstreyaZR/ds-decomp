use std::{
    fmt::{Display, LowerHex, write},
    io::Write,
    path::Path,
    vec::IntoIter,
};

use snafu::Whatever;

use crate::util::{io::read_to_string, parse::parse_u16};
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Graph {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum GraphFileType {
    NCLR,
    NSCR,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum GraphNodeTag {
    Address,
    File(GraphFileType),
    Collection,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum GraphNodeWord {
    Address(u32),
    Symbol(String),
}

impl LowerHex for GraphNodeWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Address(x) => write!(f, "{:#x}", x),
            Self::Symbol(x) => write!(f, "{x}"),
        }
    }
}

impl GraphNodeWord {
    fn from_str(source: &str) -> Self {
        match source.strip_prefix("0x") {
            Some(x) => Self::Address(u32::from_str_radix(x, 16).unwrap()),
            None => Self::Symbol(source.into()),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Address(_) => false,
            Self::Symbol(x) => x.is_empty(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct GraphNodeTags(Vec<GraphNodeTag>);

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct GraphNodes(Vec<GraphNode>);

impl IntoIterator for GraphNodes {
    type IntoIter = IntoIter<Self::Item>;
    type Item = GraphNode;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl GraphNodes {
    fn push(&mut self, item: GraphNode) {
        self.0.push(item);
    }

    fn resolve_labels(&self) -> Self {
        let mut entries = Self::default();
        let iterator = self.clone().into_iter();

        let mut node = GraphNode { name: "START".into(), ..Default::default() };

        for entry in iterator {
            node = match !entry.name.contains(".L_") {
                true => {
                    node.clean();
                    entries.push(node);
                    entry
                }
                false => node.consume_label(&entry),
            }
        }

        entries
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct GraphNode {
    name: String,
    tag: GraphNodeTags,
    byte: Vec<u16>,
    byte_as_str: Vec<String>,
    word: Vec<GraphNodeWord>,
}

impl GraphNode {
    fn from_entry(entry: &GraphEntry) -> Self {
        let name = entry.name();
        let byte = entry.byte();
        let byte_as_str = vec![entry.byte_as_str()];
        let word = vec![GraphNodeWord::from_str(&entry.word())];
        Self { name, byte, byte_as_str, word, ..Default::default() }
    }

    fn consume_entry(self, entry: &GraphEntry) -> Self {
        assert!(entry.name == None);
        let name = self.name;
        let mut byte = self.byte;
        let mut byte_as_str = self.byte_as_str;
        let mut word = self.word;
        if let Some(e_byte) = &entry.byte {
            byte.extend(e_byte);
        }
        if let Some(e_byte_str) = &entry.byte_as_str {
            byte_as_str.push(e_byte_str.to_string());
        }
        if let Some(e_word) = &entry.word {
            word.push(GraphNodeWord::from_str(&e_word.to_string()));
        }

        Self { name, byte, byte_as_str, word, ..Default::default() }
    }

    fn consume_label(&self, label: &Self) -> Self {
        let mut word = self.word.clone();
        word.extend(label.word.clone());

        Self {
            name: self.name.clone(),
            byte: self.byte.clone(),
            byte_as_str: self.byte_as_str.clone(),
            word,
            ..Default::default()
        }
    }

    /// Sorts Words, and cleans up Empty Strings / Words
    fn clean(&mut self) {
        self.byte_as_str = self
            .byte_as_str
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| x.to_string())
            .collect::<Vec<String>>();

        self.word = self
            .word
            .iter()
            .filter(|&x| !x.is_empty())
            .map(|x| x.to_owned())
            .collect::<Vec<GraphNodeWord>>();
        self.word.sort();
    }

    fn print(&self) {
        let mut s = format!("[{}]\n", self.name);

        if !self.byte.is_empty() {
            s.push_str(&format!(
                "Byte:\t{:x?}\nString:\t\"{}\"\n",
                self.byte,
                self.byte_as_str.join("")
            ));
        }
        if !self.word.is_empty() {
            s.push_str(&format!("Word:\n"));
            for i in &self.word {
                s.push_str(&format!("\t{0:x}\n", i));
            }
        }
        write!(&mut std::io::stdout(), "{}\n", s);
    }
}

/// Struct that is built by every symbol / label declaration with form `"NAME: Option<WORD>"`
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
        // Check for Label-leading Symbols
        if source[0].contains(".L_") {
            let label = true;
            let mut word = None;
            let name = Some(source[0].strip_suffix(":").unwrap().to_string());
            if source.len() > 1 {
                word = Some(source[2].to_string());
            }
            return Ok(Self { name, label, word, ..Default::default() });
        }
        // Check for Data Symbols with byte
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

            let byte_string = String::from_utf16(byte.clone().as_slice()).unwrap();
            Ok(Self { byte: Some(byte), byte_as_str: Some(byte_string), ..Default::default() }) // Check for Word Symbols with word
        } else if source[0].contains(".word") {
            let word: Option<String> = Some(source[1].into());
            Ok(Self { word, ..Default::default() })
        } else {
            let name = Some(source[0].strip_suffix(":").unwrap().to_string());
            Ok(Self { name, ..Default::default() })
        }
    }

    fn name(&self) -> String {
        if let Some(name) = &self.name {
            name.to_string()
        } else {
            String::new()
        }
    }

    fn byte_as_str(&self) -> String {
        if let Some(byte_as_str) = &self.byte_as_str {
            byte_as_str.to_string()
        } else {
            String::new()
        }
    }

    fn byte(&self) -> Vec<u16> {
        if let Some(byte) = &self.byte { byte.to_owned() } else { Vec::new() }
    }

    fn word(&self) -> String {
        if let Some(word) = &self.word {
            word.to_string()
        } else {
            String::new()
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
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct GraphEntries(Vec<GraphEntry>);

impl GraphEntries {
    fn add(&mut self, x: GraphEntry) {
        self.0.push(x)
    }

    fn collect_into_nodes(&self) -> GraphNodes {
        let mut entries = GraphNodes::default();
        let iterator = self.clone().into_iter();

        let mut node = GraphNode { name: "START".into(), ..Default::default() };

        for entry in iterator {
            node = match entry.name.is_some() {
                true => {
                    node.clean();
                    entries.push(node);
                    GraphNode::from_entry(&entry)
                }
                false => node.consume_entry(&entry),
            }
        }

        entries
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
        let buffer = read_to_string(file).unwrap();
        let lines = buffer.lines();

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
    #[allow(unused)]
    fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn from_file<P: AsRef<Path>>(file: P) /*-> Result<Self, Whatever>*/
    {
        let file = file.as_ref();
        let entries = GraphParser::entries(file).unwrap();
        let nodes = entries.collect_into_nodes();
        let nodes = nodes.resolve_labels();
        for i in nodes.into_iter().as_ref() {
            i.print();
        }
        // for i in entries {
        //     println!("{}", i);
        // }
    }
}
