use std::{collections::BTreeMap, fmt::Display, path::Path, rc::Rc, str::FromStr, vec::IntoIter};

type WordVec = Vec<Rc<GraphNodeWord>>;
type TreeTable = BTreeMap<Rc<GraphNodeWord>, Vec<GraphNode>>;
type Nodes = BTreeMap<Rc<GraphNodeWord>, GraphNode>;
type Edge = BTreeMap<Rc<GraphNodeWord>, WordVec>;

trait GraphTrait {
    fn edges_out(&self, node: Rc<GraphNodeWord>) -> WordVec;
    fn edges_in(&self, node: Rc<GraphNodeWord>) -> WordVec;
    fn add_edge(&mut self, from: Rc<GraphNodeWord>, to: Rc<GraphNodeWord>);
    fn add_edge_from_vec(&mut self, vec: Vec<(Rc<GraphNodeWord>, Rc<GraphNodeWord>)>);
    fn remove_edge(&mut self, from: Rc<GraphNodeWord>, to: Rc<GraphNodeWord>);
    fn add_node(&mut self, node: GraphNode);
    fn remove_node(&mut self, node: GraphNodeWord);
    fn is_tree(&self, node: Rc<GraphNodeWord>) -> bool;
}

use snafu::Whatever;
use strum::EnumString;

use crate::util::{io::read_to_string, parse::parse_u16};
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Graph {
    options: GraphOptions,
    nodes: Nodes,
    edges: Edge,
    grouped_nodes: Nodes,
    trees: TreeTable,
}
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct GraphOptions {}

#[derive(EnumString, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[strum(ascii_case_insensitive)]
enum GraphFileType {
    Carc, // MKDS File
    Nclr,
    Nscr,
    Bmg,
    Ncer,
    Bnll,
    Bnbl,
    Ncgr,
    Nbfc, // Banner File
    Nbfp, // Banner File
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

impl Display for GraphNodeWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        match self {
            Self::Address(num) => {
                s.push_str(&format!("{:#x}", num));
            }
            Self::Symbol(x) => {
                s.push_str(&x.to_string());
            }
        }
        f.write_str(s.as_str())
    }
}

impl Default for GraphNodeWord {
    fn default() -> Self {
        Self::Symbol(String::default())
    }
}

struct GraphNodeWordFromStrErr;

impl FromStr for GraphNodeWord {
    type Err = GraphNodeWordFromStrErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_string().into())
    }
}

impl From<String> for GraphNodeWord {
    fn from(value: String) -> Self {
        match value.strip_prefix("0x") {
            Some(x) => Self::Address(u32::from_str_radix(x, 16).unwrap()),
            None => Self::Symbol(value),
        }
    }
}
/*
impl GraphNodeWord {
    fn is_empty(&self) -> bool {
        match self {
            Self::Address(_) => false,
            Self::Symbol(x) => x.is_empty(),
        }
    }
}*/

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
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum GraphNodeTreeType {
    #[default]
    Init,
    Orphan,
    LeafSimple,
    LeafMult,
    Root,
    NodeSimple,
    NodeMult,
}

impl GraphNodes {
    fn push(&mut self, item: GraphNode) {
        self.0.push(item);
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GraphNode {
    name: Rc<GraphNodeWord>,
    tag: GraphNodeTags,
    byte: Vec<u16>,
    byte_as_str: Vec<String>,
    word: Vec<Rc<GraphNodeWord>>,
    tree_type: GraphNodeTreeType,
}

impl GraphNode {
    fn from_entry(entry: AsmEntry) -> Self {
        let name = match &entry.name {
            Some(x) => x.to_string(),
            None => String::new(),
        };
        let byte = entry.byte();
        let byte_as_str = vec![entry.byte_as_str()];
        if let Some(word) = entry.word_as_graph_node_word() {
            let word = vec![Rc::new(word)];
            return Self {
                name: Rc::new(name.into()),
                byte,
                byte_as_str,
                word,
                ..Default::default()
            };
        }
        Self { name: Rc::new(name.into()), byte, byte_as_str, ..Default::default() }
    }

    fn consume_entry(self, entry: AsmEntry) -> Self {
        assert!(entry.name.is_none(), "Entry: {:#?} did not pass no-name check", entry);
        let name = self.name;
        let mut byte = self.byte;
        let mut byte_as_str = self.byte_as_str;
        let mut word = self.word;
        if let Some(e_byte) = &entry.byte {
            byte.extend(e_byte);
        }
        if let Some(e_byte_str) = entry.byte_as_str {
            byte_as_str.push(e_byte_str);
        }
        if let Some(e_word) = entry.word {
            word.push(Rc::new(e_word.into()));
        }

        Self { name, byte, byte_as_str, word, ..Default::default() }
    }

    fn consume_label_node(self, label: Self) -> Self {
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

    fn is_file(&self) -> Option<GraphFileType> {
        let s = self.byte_as_str.join("");
        if let Some(s) =
            s.trim_end_matches([b' ' as char, '\0']).to_lowercase().split('.').next_back()
        {
            return GraphFileType::from_str(s).ok();
        }
        None
    }

    fn add_tag(&mut self, tag: GraphNodeTag) {
        self.tag.0.push(tag);
    }
}

impl GraphTrait for Graph {
    fn edges_in(&self, node: Rc<GraphNodeWord>) -> WordVec {
        if let Some(found_node) = self.nodes.get(&node) {
            self.edges
                .iter()
                .filter(|(_x, y)| y.contains(&found_node.name))
                .map(|(x, _y)| x.clone())
                .collect()
        } else {
            vec![]
        }
    }

    fn edges_out(&self, node: Rc<GraphNodeWord>) -> WordVec {
        if let Some(found_node) = self.edges.get(&node) {
            found_node.clone()
        } else {
            vec![]
        }
    }

    fn add_edge(&mut self, from: Rc<GraphNodeWord>, to: Rc<GraphNodeWord>) {
        if let Some(edge) = self.edges.get(&from) {
            let mut edge = edge.clone();
            edge.push(to);
            self.edges.insert(from, edge);
        } else {
            self.edges.insert(from, vec![to]);
        }
    }

    fn add_edge_from_vec(&mut self, vec: Vec<(Rc<GraphNodeWord>, Rc<GraphNodeWord>)>) {
        for i in vec {
            self.add_edge(i.0, i.1);
        }
    }

    fn remove_edge(&mut self, from: Rc<GraphNodeWord>, to: Rc<GraphNodeWord>) {
        if let Some(found_edge) = self.edges.get(&from) {
            let found_edge =
                found_edge.iter().filter(|x| !(**x == to)).cloned().collect();
            self.edges.insert(from, found_edge);
        }
    }

    fn add_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.name.clone(), node);
    }

    fn remove_node(&mut self, node: GraphNodeWord) {
        if self.nodes.get(&node).is_some() {
            self.nodes.remove(&node);
        }
    }

    fn is_tree(&self, _node: Rc<GraphNodeWord>) -> bool {
        todo!();
    }
}

impl Graph {
    pub fn tags(&mut self) {
        for node in self.nodes.values_mut() {
            node.apply_tags();
        }
    }

    pub fn from_files<P: AsRef<Path>>(options: GraphOptions, files: Vec<P>) -> Self {
        let mut nodes: Nodes = BTreeMap::default();

        for i in files {
            let mut entries_vec = GraphNodes::default();
            if let Ok(entries) = AsmParser::parse(i) {
                let tmp_nodes = entries.to_nodes();
                entries_vec.0.extend(tmp_nodes);
                for i in entries_vec {
                    nodes.insert(i.name.clone(), i);
                }
            }
        }

        Graph { options, nodes, ..Default::default() }
    }
}

use asm::*;

mod asm {
    use super::*;
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
    pub struct AsmEntry {
        pub(super) name: Option<String>,
        pub(super) label: bool,
        pub(super) bss: bool,
        pub(super) byte: Option<Vec<u16>>,
        pub(super) byte_as_str: Option<String>,
        pub(super) word: Option<String>,
    }

    impl AsmEntry {
        /// Assumes not to fail from input
        pub(super) fn from_str(source: &str) -> Result<Option<Self>, Whatever> {
            let source: Vec<&str> = source.split_whitespace().collect();

            // LABELS
            if source[0].starts_with(".L_") {
                if source.len() == 1 || source.len() == 4 {
                    return Ok(None);
                }
                let label = true;
                let name = None;
                let word = Some(source[2].to_string());
                Ok(Some(Self { name, label, word, ..Default::default() }))
            } else if source[0].contains(".byte") {
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
                Ok(Some(Self {
                    byte: Some(byte),
                    byte_as_str: Some(byte_string),
                    ..Default::default()
                }))
            } else if source[0].contains(".word") {
                let word: Option<String> = Some(source[1].into());
                Ok(Some(Self { word, ..Default::default() }))
            } else if source[0].starts_with("b") && !source[1].starts_with(".L_") {
                let label = true;
                let word = match source[1] {
                    "r0" | "r1" | "r2" | "r3" | "r4" | "r5" | "r6" | "r7" | "r8" | "r9" | "r10"
                    | "r11" | "lr" | "ip" | "pc" => None,
                    "r0," | "r1," | "r2," | "r3," | "r4," | "r5," | "r6," | "r7," | "r8,"
                    | "r9," | "r10," | "r11," | "lr," | "ip," | "pc," => None,
                    x => Some(x.to_string()),
                };
                if word.is_none() {
                    return Ok(None);
                }
                // println!("{word:?}");
                Ok(Some(Self { label, word, ..Default::default() }))
            } else if source.len() == 3 && source[1].contains(".space") {
                let name = Some(source[0].strip_suffix(":").unwrap().to_string());
                Ok(Some(Self { name, ..Default::default() }))
            } else {
                if let Some(name) = source[0].strip_suffix(":") {
                    if name.contains(".L_") {
                        panic!("Label Symbol Escaped");
                    }
                    Ok(Some(Self { name: Some(name.to_string()), ..Default::default() }))
                } else {
                    Ok(None)
                }
            }
        }

        pub(super) fn byte_as_str(&self) -> String {
            if let Some(byte_as_str) = &self.byte_as_str {
                byte_as_str.to_string()
            } else {
                String::new()
            }
        }

        pub(super) fn byte(&self) -> Vec<u16> {
            if let Some(byte) = &self.byte { byte.to_owned() } else { Vec::new() }
        }

        pub(super) fn word_as_graph_node_word(&self) -> Option<GraphNodeWord> {
            self.word.as_ref().map(|word| word.to_string().into())
        }
    }

    impl Display for AsmEntry {
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

    /// Simple Vector Collection for [AsmEntry]
    #[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    pub struct AsmEntries(Vec<AsmEntry>);

    impl AsmEntries {
        fn add(&mut self, x: AsmEntry) {
            self.0.push(x)
        }

        pub(super) fn to_nodes(&self) -> GraphNodes {
            let mut nodes = GraphNodes::default();

            // log::info!("{:#?}", self);

            let mut switch = false;
            let mut node = GraphNode {
                name: Rc::new(GraphNodeWord::from("GRAPH_DUMMY_START".to_string())),
                ..Default::default()
            };

            for entry in self.clone() {
                if entry.name.is_some() {
                    if switch {
                        node.clean();
                        nodes.push(node);
                    } else {
                        switch = true;
                    }
                    node = GraphNode::from_entry(entry);
                    // log::info!("From Entry{:#?}", node);
                } else {
                    // log::info!("Consumed: {:?} {:?}", node.name, entry);
                    node = node.consume_entry(entry);
                }
            }

            nodes.push(node);
            // log::info!("{:?}", nodes);
            nodes
        }
    }
    impl IntoIterator for AsmEntries {
        type IntoIter = IntoIter<Self::Item>;
        type Item = AsmEntry;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    #[derive(Debug, Default)]
    pub(super) struct AsmParser;

    /// Parses a ASM File line-by-line to [AsmEntry], creating the collection [AsmEntries]
    impl AsmParser {
        pub(super) fn parse<P: AsRef<Path>>(file: P) -> Result<AsmEntries, Whatever> {
            let file = file.as_ref();
            let buffer = read_to_string(file).unwrap();
            let lines = buffer.lines();

            // filter everything except definitions, words and bytes
            let lines: Vec<_> = lines
                .filter(|line| {
                    line.contains(":")
                        || line.contains(".word")
                        || line.contains(".byte")
                        || line.contains(".space")
                        || line.trim_start().starts_with("b")
                })
                .collect();

            let mut defs = AsmEntries::default();
            for def_line in lines {
                // log::info!("Found Line:\t {:?}", def_line);
                if let Some(def) = AsmEntry::from_str(def_line).unwrap() {
                    // log::info!("Which yielded:\t {:?}", def);
                    defs.add(def);
                }
            }

            Ok(defs)
        }
    }
}
