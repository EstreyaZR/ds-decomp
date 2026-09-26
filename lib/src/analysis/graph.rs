#![allow(dead_code)]

use std::{collections::BTreeMap, fmt::Display, path::Path, rc::Rc, str::FromStr, vec::IntoIter};

type WordVec = Vec<Rc<GraphNodeWord>>;
type Trees = BTreeMap<Rc<GraphNodeWord>, Tree>;
type Nodes = BTreeMap<Rc<GraphNodeWord>, GraphNode>;
type Edges = BTreeMap<Rc<GraphNodeWord>, WordVec>;

trait GraphTrait: Eq + Display {
    type Edge;
    type Item;
    type Indexer;

    fn edges_out(&self, node: &Self::Indexer) -> Self::Edge;
    fn edges_in(&self, node: &Self::Indexer) -> Self::Edge;

    fn add_edge(&mut self, from: &Self::Indexer, to: &Self::Indexer);
    fn remove_edge(&mut self, from: &Self::Indexer, to: &Self::Indexer);
    fn remove_related_edges(&mut self, node: &Self::Indexer);

    fn add_node(&mut self, node: Self::Item);
    fn pop_node(&mut self, node: Self::Indexer) -> Option<Self::Item>;
}

trait TreeTrait: Eq + Display {
    // fn new(name: Rc<GraphNodeWord>) -> Self;
    fn new_from_node(node: GraphNode) -> Self;
    fn get_root(&self) -> Rc<GraphNodeWord>;
    // fn set_root(&mut self, node: Rc<GraphNodeWord>) -> Rc<GraphNodeWord>;
    fn add_node(&mut self, node: GraphNode) -> Rc<GraphNodeWord>;
    fn consume_tree(&mut self, tree: Self);
    // fn consume_node(&mut self, node: GraphNode) -> Rc<GraphNodeWord>;
    // fn pop_node(&mut self, node: Rc<GraphNodeWord>) -> Option<GraphNode>;
}
#[derive(Default, Debug, Eq, PartialEq, PartialOrd, Ord, Clone)]
struct Tree {
    root: GraphNode,
    nodes: Nodes,
    // edges: Edges,
}

impl Display for Tree {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl TreeTrait for Tree {
    fn new_from_node(node: GraphNode) -> Self {
        Self { root: node.clone(), ..Default::default() }
    }

    fn add_node(&mut self, node: GraphNode) -> Rc<GraphNodeWord> {
        let name = node.name.clone();
        self.nodes.insert(name.clone(), node);
        name
    }

    fn get_root(&self) -> Rc<GraphNodeWord> {
        todo!()
    }

    fn consume_tree(&mut self, tree: Self) {
        self.add_node(tree.root);
        for (name, node) in tree.nodes {
            self.nodes.insert(name.clone(), node.clone());
        }
    }
}

use snafu::Whatever;
use strum::EnumString;

use crate::util::{io::read_to_string, parse::parse_u16};
#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Graph {
    options: GraphOptions,
    nodes: Nodes,
    edges_up: Edges,
    edges_down: Edges,
    grouped_nodes: Nodes,
    trees: Trees,
}

impl Display for Graph {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct GraphOptions {
    pub debug: bool,
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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
struct GraphNodeTags(Vec<GraphNodeTag>);

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
    TreeRoot,
    TreeInit,
    TreeSimple,
    TreeMult,
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
    word: WordVec,
    called_by: WordVec,
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

    fn has_tree_type(&self, tree_type: GraphNodeTreeType) -> bool {
        self.tree_type == tree_type
    }
}

impl GraphTrait for Graph {
    type Edge = Edges;
    type Indexer = Rc<GraphNodeWord>;
    type Item = GraphNode;

    // For called_by relations
    fn edges_in(&self, node: &Self::Indexer) -> Self::Edge {
        let mut ret = BTreeMap::new();

        if let Some(found_node) = self.edges_up.get(node) {
            ret.insert(node.clone(), found_node.clone());
        } else {
            ret.insert(node.clone(), vec![]);
        }
        ret
    }

    // For word relations
    fn edges_out(&self, node: &Self::Indexer) -> Self::Edge {
        let mut ret = BTreeMap::new();

        if let Some(found_node) = self.edges_down.get(node) {
            ret.insert(node.clone(), found_node.clone());
        } else {
            ret.insert(node.clone(), vec![]);
        }
        ret
    }

    fn add_edge(&mut self, from: &Self::Indexer, to: &Self::Indexer) {
        if let Some(edge) = self.edges_down.get(from) {
            let mut edge = edge.clone();
            edge.push(to.clone());
            self.edges_down.insert(from.clone(), edge);
        } else {
            self.edges_down.insert(from.clone(), vec![to.clone()]);
        }
        if let Some(edge) = self.edges_up.get(to) {
            let mut edge = edge.clone();
            edge.push(to.clone());
            self.edges_up.insert(to.clone(), edge);
        } else {
            self.edges_up.insert(to.clone(), vec![from.clone()]);
        }
    }

    fn remove_related_edges(&mut self, node: &Self::Indexer) {
        self.edges_up.remove(node);
        self.edges_down.remove(node);
    }

    fn remove_edge(&mut self, from: &Self::Indexer, to: &Self::Indexer) {
        if let Some(found_edge) = self.edges_down.get(from) {
            let found_edge: WordVec =
                found_edge.iter().filter(|x| !(**x == to.clone())).cloned().collect();
            self.edges_down.insert(from.clone(), found_edge.clone());
        }
        if let Some(found_edge) = self.edges_up.get(to) {
            let found_edge: WordVec =
                found_edge.iter().filter(|x| !(**x == from.clone())).cloned().collect();
            self.edges_up.insert(to.clone(), found_edge.clone());
        }
    }

    fn add_node(&mut self, node: Self::Item) {
        self.nodes.insert(node.name.clone(), node);
    }

    fn pop_node(&mut self, node: Self::Indexer) -> Option<Self::Item> {
        self.nodes.remove(&node)
    }
}

impl Graph {
    fn nodes_by_tree_type(&self, tree_type: GraphNodeTreeType) -> WordVec {
        self.nodes.values().filter(|x| x.tree_type == tree_type).map(|x| x.name.clone()).collect()
    }

    fn edges_out_by_tree_type(&self, tree_type: GraphNodeTreeType) -> Edges {
        let mut ret = BTreeMap::new();

        let found = self.nodes_by_tree_type(tree_type);

        for i in found {
            if let Some(found_node) = self.edges_down.get(&i) {
                ret.insert(i.clone(), found_node.clone());
            } else {
                ret.insert(i.clone(), vec![]);
            }
        }
        ret
    }

    fn edges_in_by_tree_type(&self, tree_type: GraphNodeTreeType) -> Edges {
        let mut ret: Edges = BTreeMap::new();

        let found = self.nodes_by_tree_type(tree_type);

        for i in found {
            if let Some(found_node) = self.edges_up.get(&i) {
                ret.insert(i.clone(), found_node.clone());
            } else {
                ret.insert(i.clone(), vec![]);
            }
        }
        ret
    }

    fn lookup_edges_in(
        lookup_nodes: &Nodes,
        lookup_edges: &Edges,
        node: &Rc<GraphNodeWord>,
    ) -> WordVec {
        if let Some(found_node) = lookup_nodes.get(node) {
            lookup_edges
                .iter()
                .filter(|(_x, y)| y.contains(&found_node.name))
                .map(|(x, _y)| x.clone())
                .collect()
        } else {
            vec![]
        }
    }

    fn lookup_edges_out(lookup_edges: &Edges, node: Rc<GraphNodeWord>) -> WordVec {
        if let Some(found_node) = lookup_edges.get(&node) {
            found_node.clone()
        } else {
            vec![]
        }
    }

    pub fn apply_tags(&mut self) {
        for node in self.nodes.values_mut() {
            node.apply_tags();
        }
    }

    pub fn init_called_by(&mut self) {
        let lookup_edges = self.edges_up.clone();
        for node in self.nodes.values_mut() {
            if let Some(found_callers) = lookup_edges.get(&node.name) {
                node.called_by = found_callers.clone();
            }
        }
    }

    pub fn init_edges(&mut self) {
        let lookup_nodes = self.nodes.clone();
        for node in lookup_nodes.values() {
            if node.word.is_empty() {
                continue;
            } else {
                let calls = node.word.clone();
                for called in calls {
                    self.add_edge(&node.name.clone(), &called.clone());
                }
            }
        }
    }

    pub fn init_tree_status(&mut self) {
        let mut statistics = (0, 0, 0, 0, 0, 0);
        for node in self.nodes.values_mut() {
            if node.tree_type == GraphNodeTreeType::TreeMult
                || node.tree_type == GraphNodeTreeType::TreeInit
            {
                node.tree_type = match node.called_by.len() {
                    0 => GraphNodeTreeType::TreeRoot,
                    1 => GraphNodeTreeType::TreeSimple,
                    2.. => GraphNodeTreeType::TreeMult,
                }
            } else {
                node.tree_type = match (node.called_by.len(), node.word.len()) {
                    (0, 0) => {
                        statistics.0 += 1;
                        GraphNodeTreeType::Orphan
                    }
                    (1, 0) => {
                        statistics.1 += 1;
                        GraphNodeTreeType::LeafSimple
                    }
                    (2.., 0) => {
                        statistics.2 += 1;
                        GraphNodeTreeType::LeafMult
                    }
                    (0, 1..) => {
                        statistics.3 += 1;
                        GraphNodeTreeType::Root
                    }
                    (1, 1..) => {
                        statistics.4 += 1;
                        GraphNodeTreeType::NodeSimple
                    }
                    (2.., 1..) => {
                        statistics.5 += 1;
                        GraphNodeTreeType::NodeMult
                    }
                }
            }
        }
        if self.options.debug {
            log::info!("Statistics: {:?}", statistics);
        }
    }

    pub fn find_trees(&mut self) {
        let nodes_in_graph = self.nodes.len();
        let edges = self.edges_down.clone();

        for (node, vector) in edges.iter() {
            let status_vec: Vec<_> = vector
                .iter()
                .filter(|x| {
                    if let Some(real_node) = self.nodes.get(x.clone()) {
                        real_node.has_tree_type(GraphNodeTreeType::LeafSimple)
                            || real_node.has_tree_type(GraphNodeTreeType::TreeSimple)
                    } else {
                        false
                    }
                })
                .collect();
            if !status_vec.is_empty()
                && let Some(tree_node) = self.pop_node(node.clone())
            {
                let mut tree_node = tree_node.clone();

                tree_node.tree_type = GraphNodeTreeType::TreeInit;
                let mut tree = Tree::new_from_node(tree_node.clone());

                tree_node.word.clear();
                self.add_node(tree_node);

                for leaf in vector {
                    if let Some(leaf) = self.pop_node(leaf.clone()) {
                        if leaf.tree_type == GraphNodeTreeType::TreeSimple {
                            if let Some(popped_tree) = self.trees.remove(&leaf.name) {
                                tree.consume_tree(popped_tree);
                                self.remove_related_edges(&leaf.name);
                            }
                        } else {
                            tree.add_node(leaf.clone());
                            self.remove_related_edges(&leaf.name);
                        }
                    }
                }
                self.trees.insert(tree.root.name.clone(), tree);
            }
        }
        let nodes_in_graph_new = self.nodes.len();
        log::info!(
            "{} Nodes remain from {} scanned in Source Assembly\nGraph has {} Trees",
            nodes_in_graph_new,
            nodes_in_graph,
            self.trees.len()
        );
    }

    pub fn from_files<P: AsRef<Path>>(options: GraphOptions, files: Vec<P>) -> Self {
        let mut nodes: Nodes = BTreeMap::new();

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
