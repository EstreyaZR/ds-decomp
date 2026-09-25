use std::{
    collections::BTreeMap, fmt::Display, io::Write, path::Path, rc::Rc, str::FromStr, vec::IntoIter,
};

use snafu::Whatever;
use strum::EnumString;

use crate::util::{io::read_to_string, parse::parse_u16};
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Graph {
    options: AsmParserOptions,
    nodes: BTreeMap<Rc<GraphNodeWord>, GraphNode>,
    grouped_nodes: BTreeMap<Rc<GraphNodeWord>, GraphNode>,
}
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct AsmParserOptions {
    split_into_files: bool,
}

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
    // Symbol only found after reading all references
    GeneratedByGraph,
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

impl GraphNodeWord {
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
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum GraphNodeTreeType {
    #[default]
    Init,
    Orphan,
    LeafSimple,
    LeafMult,
    Root,
    Node,
}

impl GraphNodes {
    fn push(&mut self, item: GraphNode) {
        self.0.push(item);
    }

    pub fn resolve_labels(self) -> Self {
        let mut entries = Self::default();
        let mut node = GraphNode {
            name: Rc::new("GRAPH_DUMMY_START".to_string().into()),
            ..Default::default()
        };
        let mut node_switch: bool = false; // turns true if Start has been skipped, workaround for the compiler since it wont allow NULL-ptr before the for-loop.
        for entry in self.into_iter() {
            node = match entry.label {
                false => {
                    if !node_switch {
                        node_switch = true;
                        continue;
                    }
                    node.clean();
                    entries.push(node);
                    entry
                }
                true => node.consume_label_node(entry),
            }
        }
        node.clean();
        entries.push(node);
        entries
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GraphNode {
    name: Rc<GraphNodeWord>,
    level: Option<u32>,
    label: bool,
    tag: GraphNodeTags,
    byte: Vec<u16>,
    byte_as_str: Vec<String>,
    word: Vec<Rc<GraphNodeWord>>,
    called_by: Vec<Rc<GraphNodeWord>>,
    tree_type: GraphNodeTreeType,
}

impl GraphNode {
    fn from_entry(entry: AsmEntry) -> Self {
        let name = match &entry.name {
            Some(x) => x.to_string(),
            None => String::new(),
        };
        let label = entry.label;
        let byte = entry.byte();
        let byte_as_str = vec![entry.byte_as_str()];
        if let Some(word) = entry.word_as_graph_node_word() {
            let word = vec![Rc::new(word)];
            return Self {
                name: Rc::new(name.into()),
                label,
                byte,
                byte_as_str,
                word,
                ..Default::default()
            };
        }
        Self { name: Rc::new(name.into()), label, byte, byte_as_str, ..Default::default() }
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

        self.word = self.word.iter().map(|x| x.to_owned()).collect::<Vec<Rc<GraphNodeWord>>>();
        self.word.sort();
    }

    fn print(&self) {
        let mut s = format!("[{}]\t{:?}\n", self.name, self.level);

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
                s.push_str(&format!("\t{}\n", *i)); //
            }
        }
        if !self.called_by.is_empty() {
            s.push_str("[CalledBy]\n");
            for i in &self.called_by {
                s.push_str(&format!("\t{}\n", *i));
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
        if let Some(s) = s
            .trim_end_matches([b' ' as char, '\0'])
            .to_lowercase()
            .split('.')
            .next_back()
        {
            return GraphFileType::from_str(s).ok();
        }
        None
    }

    fn children(&self) -> Vec<Rc<GraphNodeWord>> {
        let mut calls = Vec::new();
        for i in &self.word {
            calls.push(i.clone())
        }
        calls
    }

    fn called_by(&self) -> Vec<Rc<GraphNodeWord>> {
        let mut calls = Vec::new();
        for i in &self.called_by {
            calls.push(i.to_owned())
        }
        calls
    }

    fn add_tag(&mut self, tag: GraphNodeTag) {
        self.tag.0.push(tag);
    }

    fn update_dependencies(&mut self, lookup_table: &BTreeMap<Rc<GraphNodeWord>, GraphNode>) {
        let mut dependencies = Vec::new();
        for i in self.children() {
            if let Some(rc) = lookup_table.get(&i) {
                dependencies.push(rc.name.clone());
            } else {
                dependencies.push(i);
            }
        }
        self.word = dependencies;
    }

    fn update_tree(&mut self) {
        self.tree_type = match (self.called_by.len(), self.word.len()) {
            (0, 0) => GraphNodeTreeType::Orphan,
            (1, 0) => GraphNodeTreeType::LeafSimple,
            (2.., 0) => GraphNodeTreeType::LeafMult,
            (0, 1..) => GraphNodeTreeType::Root,
            (1.., 1..) => GraphNodeTreeType::Node,
        };
    }
}

impl Graph {
    pub fn update_rc(&mut self) {
        let lookup_table = self.nodes.clone();
        for node in self.nodes.values_mut() {
            node.update_dependencies(&lookup_table);
        }
    }

    pub fn tags(&mut self) {
        for node in self.nodes.values_mut() {
            node.apply_tags();
        }
    }

    pub fn print(&self) {
        for i in self.nodes.values() {
            i.print()
        }
    }

    pub fn group_print(&self) {
        for i in self.grouped_nodes.values() {
            i.print()
        }
    }

    pub fn called_by_debug(&mut self) {
        for i in &self.nodes {
            println!("{}\t{}", i.1.called_by.len(), i.0);
        }
    }

    pub fn called_by(&mut self) {
        let mut callee_and_callers: BTreeMap<Rc<GraphNodeWord>, Vec<Rc<GraphNodeWord>>> =
            BTreeMap::default();
        for node in self.nodes.values() {
            for i in node.children() {
                if let Some(child) = callee_and_callers.get(&i) {
                    let mut child = child.clone();
                    child.push(i.clone());
                    callee_and_callers.insert(i, child);
                } else {
                    callee_and_callers.insert(i, vec![node.name.clone()]);
                }
            }
        }
        for (name, node) in self.nodes.iter_mut() {
            if let Some(callers) = callee_and_callers.get(name) {
                node.called_by = callers.clone();
            }
        }
        log::info!("Added Called By");
    }

    fn init_deps(&mut self) {
        for node in self.nodes.values_mut() {
            node.update_tree();
        }
    }

    fn group_iterate(
        mut nodes: BTreeMap<Rc<GraphNodeWord>, GraphNode>,
    ) -> BTreeMap<Rc<GraphNodeWord>, GraphNode> {
        let lookup_table = nodes.clone();
        for node in nodes.values_mut() {
            node.word = node
                .word
                .iter()
                .filter(|&x| {
                    let mut cond = false;
                    if let Some(child) = lookup_table.get(x) {
                        cond = match child.tree_type {
                            GraphNodeTreeType::LeafSimple
                            | GraphNodeTreeType::Init
                            | GraphNodeTreeType::Orphan => false,
                            GraphNodeTreeType::LeafMult
                            | GraphNodeTreeType::Root
                            | GraphNodeTreeType::Node => true,
                        };
                    };
                    cond
                }).cloned()
                .collect();
            node.update_tree();
        }
        nodes
            .iter()
            .filter(|(_x, y)| {
                y.tree_type == GraphNodeTreeType::Root
                    || y.tree_type == GraphNodeTreeType::LeafMult
                    || y.tree_type == GraphNodeTreeType::Node
            })
            .map(|(x, y)| (x.clone(), y.clone()))
            .collect()
    }

    pub fn group(&mut self) {
        self.init_deps();
        let mut nodes_mut: BTreeMap<_, _> =
            self.nodes.clone().into_iter().map(|(x, y)| (x, y.clone())).collect();

        let mut counter = 10;
        while counter > 0 {
            log::info!("Grouping Iteration {counter}");
            let new_nodes = Self::group_iterate(nodes_mut);
            counter -= 1;
            nodes_mut = new_nodes.clone();
        }

        log::info!("Collected Groups");
        let mut counter_collected = 0;
        for (name, after_grouping) in nodes_mut.iter() {
            if let Some(before_grouping) = self.nodes.get(name)
                && after_grouping != before_grouping {
                    counter_collected += 1;
                }
        }
        log::info!("{} Nodes differ from the Source Collection", counter_collected);
        log::info!(
            "With {} Total Symbols, and {} true Graph Nodes",
            self.nodes.len(),
            nodes_mut.len()
        );
        self.grouped_nodes = nodes_mut;
    }

    pub fn from_files<P: AsRef<Path>>(options: AsmParserOptions, files: Vec<P>) -> Self {
        let mut entries_vec = GraphNodes::default();
        let mut nodes: BTreeMap<Rc<GraphNodeWord>, GraphNode> = BTreeMap::default();

        for i in files.iter() {
            let entries = AsmParser::parse(i, &options).unwrap();
            // for i in entries {
            //     if i.label
            // }
            let tmp_nodes = entries.to_nodes();
            let nodes_o = tmp_nodes.resolve_labels();
            entries_vec.0.extend(nodes_o);
        }

        for i in entries_vec {
            nodes.insert(i.name.clone(), i);
        }

        Graph { options, nodes, grouped_nodes: BTreeMap::new() }
    }
}

use asm::*;

mod asm {
    use super::*;
    /// Struct that is built by every symbol / label declaration with form `"NAME: Option<WORD>"`
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
    pub struct AsmEntry {
        pub(super) name: Option<String>,
        pub(super) label: bool,
        /// None: Symbol is a Label to jump to, used in goto, etc...
        /// Some: Contains a Word Directive
        ///
        /// Example of Behaviour:
        /// ```
        /// ".L_0x1234:" => None,
        /// ".L_0x6767: .word data_importantData" => Some(data_importantData),
        /// ```
        pub(super) byte: Option<Vec<u16>>,
        pub(super) byte_as_str: Option<String>,
        // TODO: Add Kind of Word; e.g. Numerical or Symbol
        pub(super) word: Option<String>,
    }

    impl AsmEntry {
        /// Assumes not to fail from input
        pub(super) fn from_str(source: &str) -> Result<Option<Self>, Whatever> {
            let source: Vec<&str> = source.split_whitespace().collect();
            if source.len() == 3 && source[1].contains(".space") {
                let name = Some(source[0].strip_suffix(":").unwrap().to_string());
                return Ok(Some(Self { name, ..Default::default() }));
            }
            // Check for Label-leading Symbols
            if source[0].contains(".L_") {
                if source.len() == 1 || source.len() == 4 {
                    return Ok(None);
                }
                let label = true;
                let name = Some(source[0].strip_suffix(":").unwrap().to_string());
                let word = Some(source[2].to_string());
                return Ok(Some(Self { name, label, word, ..Default::default() }));
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
                Ok(Some(Self {
                    byte: Some(byte),
                    byte_as_str: Some(byte_string),
                    ..Default::default()
                }))
            } else if source[0].contains(".word") {
                let word: Option<String> = Some(source[1].into());
                Ok(Some(Self { word, ..Default::default() }))
            } else if source[0].starts_with("b")
                && !source[1].starts_with(".L_")
                && (!source[0].starts_with("bx") && source.len() == 1)
            {
                let label = true;
                let word = match source[1] {
                    "r0" | "r1" | "r2" | "r3" | "r4" | "r5" | "r6" | "r7" | "r8" | "lr" | "ip"
                    | "pc" => None,
                    x => Some(x.to_string()),
                };
                println!("{word:#?}");
                Ok(Some(Self { label, word, ..Default::default() }))
            } else {
                if let Some(name) = source[0].strip_suffix(":") {
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
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
    pub struct AsmEntries(Vec<AsmEntry>);

    impl AsmEntries {
        fn add(&mut self, x: AsmEntry) {
            self.0.push(x)
        }

        pub(super) fn to_nodes(self) -> GraphNodes {
            let mut entries = GraphNodes::default();

            let mut node = GraphNode {
                name: Rc::new(GraphNodeWord::from("GRAPH_DUMMY_START".to_string())),
                ..Default::default()
            };

            for entry in self.into_iter() {
                node = match entry.name.is_some() {
                    true => {
                        node.clean();
                        entries.push(node);
                        GraphNode::from_entry(entry)
                    }
                    false => node.consume_entry(entry),
                }
            }

            entries.push(node);

            entries
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
        pub(super) fn parse<P: AsRef<Path>>(
            file: P,
            _options: &AsmParserOptions,
        ) -> Result<AsmEntries, Whatever> {
            let file = file.as_ref();
            let buffer = read_to_string(file).unwrap();
            let lines = buffer.lines();

            // filter everything except definitions, words and bytes
            let lines: Vec<_> = lines
                .filter(|line| {
                    line.contains(":")
                        | line.contains(".word")
                        | line.contains(".byte")
                        | line.contains(".space")
                        | line.trim_start().starts_with("b")
                })
                .collect();

            let mut defs = AsmEntries::default();
            for def_line in lines {
                if let Some(def) = AsmEntry::from_str(def_line).unwrap() {
                    defs.add(def);
                }
            }

            Ok(defs)
        }
    }
}
