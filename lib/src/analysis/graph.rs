use std::{
    collections::BTreeMap,
    fmt::{Display, LowerHex},
    io::Write,
    path::Path,
    str::FromStr,
    vec::IntoIter,
};

use snafu::Whatever;
use strum::EnumString;

use crate::util::{io::read_to_string, parse::parse_u16};
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Graph {}

#[allow(unused)]
#[derive(EnumString, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum GraphFileType {
    Nclr,
    Nscr,
    Bmg,
    Ncer,
    Bnll,
    Bnbl,
    Ncgr,
}
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum GraphNodeStatus {
    /// Unprocessed
    #[default]
    Init,
    /// Only called by others
    Leaf,
    /// Has Children, no known Level.
    NodeUnkLevel,
    /// Alternative to [GraphNodeStatus::NodeUnkLevel], to stop LevelAnalysis
    Node,
    /// Unused Symbol?
    Orphan,
    /// TODO: Level of distance to a leaf, where a node directly next to a leaf child is 1.
    NodeWithLevel(u32),
    /// Has gone through Tagging,
    Tagged,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum GraphNodeTag {
    Address,
    File(GraphFileType),
    Collection,
    // Symbol only found after reading all references
    GeneratedByGraph,
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
struct GraphNodeTags(Vec<GraphNodeTag>);

impl GraphNodeTags {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for GraphNodeTags {
    type IntoIter = IntoIter<Self::Item>;
    type Item = GraphNodeTag;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GraphNodes(pub Vec<GraphNode>);

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

    pub fn resolve_labels(&self) -> Self {
        let mut entries = Self::default();
        let iterator = self.clone().into_iter();

        let mut node = GraphNode { name: "START".into(), ..Default::default() };
        let mut node_switch: bool = false; // turns true if Start has been skipped, workaround for the compiler since it wont allow NULL-ptr before the for-loop.
        for entry in iterator {
            node = match !entry.name.contains(".L_") {
                true => {
                    if !node_switch {
                        node_switch = true;
                        continue;
                    }
                    node.clean();
                    entries.push(node);
                    entry
                }
                false => node.consume_label_node(&entry),
            }
        }
        node.clean();
        entries.push(node);
        entries
    }

    #[deprecated]
    fn apply_tags(self) -> Self {
        let iterator = self.into_iter();

        let mut nodes = Self::default();

        for mut node in iterator {
            if node.is_collection() {
                node.add_tag(GraphNodeTag::Collection);
            } else if node.is_address() {
                node.add_tag(GraphNodeTag::Address);
            } else if let Some(file_type) = node.is_file() {
                node.add_tag(GraphNodeTag::File(file_type));
            }

            node.update_status(GraphNodeStatus::Tagged);
            nodes.push(node);
        }

        nodes
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GraphNode {
    name: String,
    tag: GraphNodeTags,
    byte: Vec<u16>,
    byte_as_str: Vec<String>,
    word: Vec<GraphNodeWord>,
    called_by: Vec<GraphNodeWord>,
    status: GraphNodeStatus,
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
        assert!(entry.name.is_none(), "Entry: {:#?} did not pass no-name check", entry);
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

    fn consume_label_node(&self, label: &Self) -> Self {
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
        let mut s = format!("[{}]\t{:?}\n", self.name, self.status);

        if !self.tag.is_empty() {
            s.push_str("[Tags]");
            for i in self.tag.clone() {
                s.push_str(&format!("\t{:?}\n", i));
            }
        }

        if !self.byte.is_empty() {
            s.push_str(&format!("[Data]\tSize:{:#x}\n", self.byte_size()));
            s.push_str(&format!(
                "\tByte:\t{:x?}\n\tString:\t\"{}\"\n",
                self.byte,
                self.byte_as_str.join("")
            ));
        }
        if !self.word.is_empty() {
            s.push_str("[Word]\n");
            for i in &self.word {
                s.push_str(&format!("\t{0:x}\n", i)); //
            }
        }
        if !self.called_by.is_empty() {
            s.push_str("[CalledBy]\n");
            for i in &self.called_by {
                s.push_str(&format!("\t{0:x}\n", i));
            }
        }
        writeln!(&mut std::io::stdout(), "{}", s).unwrap();
    }

    /// Takes care of actually tagging [GraphNode],
    /// please add calls for your tag check functions here,
    /// and your tags to [GraphNodeTag]
    fn apply_tags(&mut self) {
        if self.is_collection() {
            self.add_tag(GraphNodeTag::Collection);
        } else if self.is_address() {
            self.add_tag(GraphNodeTag::Address);
        } else if let Some(file_type) = self.is_file() {
            self.add_tag(GraphNodeTag::File(file_type));
        }

        self.update_status(GraphNodeStatus::Tagged);
    }

    /// Updates the [GraphNodeStatus] to the Level of Distance to a Leaf
    fn apply_level(&mut self, nodes_btreemap: &BTreeMap<String, GraphNode>) {
        if self.status == GraphNodeStatus::Tagged || self.status == GraphNodeStatus::NodeUnkLevel {
            if self.word.is_empty() {
                if self.called_by.is_empty() {
                    self.update_status(GraphNodeStatus::Orphan);
                } else {
                    self.update_status(GraphNodeStatus::Leaf);
                }
            } else {
                let mut level: u32 = 0;
                for child_node in self.word.clone() {
                    if let GraphNodeWord::Symbol(child_node) = child_node {
                        if let Some(child_node) = nodes_btreemap.get(&child_node) {
                            let child_level: u32 = match child_node.status {
                                GraphNodeStatus::Leaf | GraphNodeStatus::Orphan => 1,
                                GraphNodeStatus::NodeWithLevel(x) => x,
                                GraphNodeStatus::Tagged
                                | GraphNodeStatus::Init
                                | GraphNodeStatus::NodeUnkLevel => 0,
                                GraphNodeStatus::Node => u32::MAX,
                            };
                            level = u32::min(child_level, level);
                        } else {
                            continue;
                        }
                    }
                }
                match level {
                    0 => self.update_status(GraphNodeStatus::NodeUnkLevel),
                    1.. => self.update_status(GraphNodeStatus::NodeWithLevel(level)),
                }
            }
        }
    }

    pub fn byte_size(&self) -> usize {
        self.byte.len()
    }

    pub fn byte_are_padding(&self) -> bool {
        for &i in &self.byte {
            if i != 0 {
                return false;
            }
        }
        true
    }

    pub fn is_collection(&self) -> bool {
        !&self.word.is_empty()
            && self.word.len() != 1
            && (self.byte.is_empty() || self.byte_are_padding())
    }

    pub fn is_address(&self) -> bool {
        !&self.word.is_empty() && self.byte.is_empty() && self.word.len() == 1
    }

    pub fn is_file(&self) -> Option<GraphFileType> {
        let str = &self.byte_as_str.join("");
        let s: Vec<_> = str.split(".").collect();
        if let Some(last) = s.last() {
            return GraphFileType::from_str(last).ok();
        }
        None
    }

    fn children(&self) -> Vec<GraphNodeWord> {
        let mut calls = Vec::new();
        for i in &self.word {
            calls.push(i.to_owned())
        }
        calls
    }

    pub fn update_status(&mut self, status: GraphNodeStatus) {
        self.status = status;
    }

    pub fn add_tag(&mut self, tag: GraphNodeTag) {
        self.tag.0.push(tag);
    }
}

/// Struct that is built by every symbol / label declaration with form `"NAME: Option<WORD>"`
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct GraphEntry {
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
                // println!("{:?}", &word);
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
pub struct GraphEntries(Vec<GraphEntry>);

impl GraphEntries {
    fn add(&mut self, x: GraphEntry) {
        self.0.push(x)
    }

    pub fn to_nodes(&self) -> GraphNodes {
        let mut entries = GraphNodes::default();
        let iterator = self.clone().into_iter();

        let mut node = GraphNode { name: "GRAPH_DUMMY_START".into(), ..Default::default() };

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

        entries.push(node);

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

pub struct GraphParser {}

/// Parses a ASM File line-by-line to [GraphEntry], creating the collection [GraphEntries]
impl GraphParser {
    pub fn parse<P: AsRef<Path>>(file: P) -> Result<GraphEntries, Whatever> {
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
    pub fn with_graph_nodes(nodes: GraphNodes)
    /*-> Result<Self, Whatever>*/
    {
        let mut nodes_btreemap: BTreeMap<String, GraphNode> = BTreeMap::new();
        let mut caller_to_callee: BTreeMap<String, Vec<GraphNodeWord>> = BTreeMap::new();
        let mut callee_to_caller: BTreeMap<GraphNodeWord, Vec<String>> = BTreeMap::new();

        // for i in nodes.clone() {
        //     i.print();
        // }

        for i in nodes.clone().into_iter() {
            let children = i.children();
            caller_to_callee.insert(i.name.clone(), children.clone());
            for child in children {
                if let Some(caller) = callee_to_caller.get(&child) {
                    let mut caller = caller.clone();
                    caller.push(i.name.clone());
                    callee_to_caller.insert(child.to_owned(), caller);
                } else {
                    let caller = vec![i.name.clone()];
                    callee_to_caller.insert(child.to_owned(), caller);
                }
            }
        }
        for i in nodes.into_iter().as_ref() {
            nodes_btreemap.insert(i.name.clone(), i.clone());
        }
        let mut unk_symbol_number = 1;
        for (callee, caller) in callee_to_caller {
            if let GraphNodeWord::Symbol(callee) = callee {
                let caller = caller.iter().map(|x| GraphNodeWord::Symbol(x.to_string())).collect();

                if let Some(node) = nodes_btreemap.get(&callee) {
                    let mut node = node.clone();
                    node.called_by = caller;
                    nodes_btreemap.insert(node.name.clone(), node);
                } else {
                    let name = format!("UnknownSymbol_{unk_symbol_number}").to_string();
                    let tags = GraphNodeTags(vec![GraphNodeTag::GeneratedByGraph]);
                    nodes_btreemap.insert(name.clone(), GraphNode {
                        name,
                        tag: tags,
                        status: GraphNodeStatus::Leaf,
                        called_by: caller,
                        ..Default::default()
                    });
                    unk_symbol_number += 1;
                }
            }
        }

        for node in nodes_btreemap.values_mut() {
            node.apply_tags();
        }

        let mut counter: usize = 0;

        loop {
            let btreemap_last_iteration = nodes_btreemap.clone();
            if counter < 50 {
                for i in nodes_btreemap.values_mut() {
                    i.apply_level(&btreemap_last_iteration);
                }
            } else {
                break;
            }
            counter += 1;
        }
        for (_, i) in nodes_btreemap {
            i.print();
        }

        // for i in entries {
        //     println!("{}", i);
        // }
    }
}
