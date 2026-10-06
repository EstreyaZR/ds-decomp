mod addresses;
mod asm;
mod node;
mod tree;
use std::{
    collections::BTreeMap,
    fmt::Display,
    io::{BufWriter, Write},
    path::Path,
    rc::Rc,
    str::FromStr,
    vec::IntoIter,
};

pub(crate) use addresses::*;
pub(crate) use asm::*;
pub(crate) use node::*;
use snafu::Whatever;
use strum::EnumString;
pub(crate) use tree::*;

use crate::util::io::*;

type WordVec = Vec<Rc<AsmWord>>;
type Trees = BTreeMap<Rc<AsmWord>, Tree>;
type Nodes = BTreeMap<Rc<AsmWord>, GraphNode>;
type Edges = BTreeMap<Rc<AsmWord>, WordVec>;

#[allow(unused)]
pub trait GraphTrait {
    type Edge: Eq + Ord;
    type Item;
    type Indexer: Eq + Ord;

    #[allow(unused)]
    fn edges_out(&self, node: &Self::Indexer) -> Self::Edge;
    fn edges_in(&self, node: &Self::Indexer) -> Self::Edge;

    fn add_edge(&mut self, from: &Self::Indexer, to: &Self::Indexer);
    fn remove_edge(&mut self, from: &Self::Indexer, to: &Self::Indexer);
    fn remove_related_edges(&mut self, node: &Self::Indexer);

    fn add_node(&mut self, node: Self::Item);
    fn pop_node(&mut self, node: Self::Indexer) -> Option<Self::Item>;
}

use crate::util::io::read_to_string;
#[derive(Default)]
pub struct Graph {
    options: GraphOptions,
    files: Vec<AsmFile>,
    nodes: Nodes,
    edges_up: Edges,
    edges_down: Edges,
    trees: Trees,
}

#[derive(Default, Copy, Clone)]
pub struct GraphOptions {
    pub debug: bool,
}

impl GraphTrait for Graph {
    type Edge = Edges;
    type Indexer = Rc<AsmWord>;
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

    fn add_edge(&mut self, caller: &Self::Indexer, callee: &Self::Indexer) {
        if let Some(found_callee) = self.edges_down.get(caller) {
            let mut found_callee = found_callee.clone();
            found_callee.push(callee.clone());
            self.edges_down.insert(caller.clone(), found_callee);
        } else {
            self.edges_down.insert(caller.clone(), vec![callee.clone()]);
        }
        if let Some(found_callers) = self.edges_up.get(callee) {
            let mut found_callers = found_callers.clone();
            found_callers.push(caller.clone());
            self.edges_up.insert(callee.clone(), found_callers);
        } else {
            self.edges_up.insert(callee.clone(), vec![caller.clone()]);
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
        self.nodes.insert(node.name(), node);
    }

    fn pop_node(&mut self, node: Self::Indexer) -> Option<Self::Item> {
        self.nodes.remove(&node)
    }
}

impl Graph {
    pub fn init_nodes(&mut self) {
        self.init_edges();
        log::info!("Finished InitEdges");

        self.init_node_connections();
        log::info!("Finished init_in_connections");

        self.init_loops();
        log::info!("Finished init_loops");

        // self.resolve_loops();

        self.init_bytes_to_string();

        self.apply_tags();
        log::info!("Finished ApplyTags");

        // For later
        // self.parse_build_info();
    }

    pub fn parse_build_info(&self) {
        let build_info = Rc::new(AsmWord::from_str("BuildInfo").unwrap());
        if let Some(build_info_node) = self.nodes.get(&build_info) {
            build_info_node.parse_build_info();
        } else {
            log::error!("Couldn't find BuildInfo");
        }
    }

    fn resolve_loops(&mut self) {
        self.nodes.values_mut().for_each(|x| x.resolve_loops());
    }

    fn init_bytes_to_string(&mut self) {
        self.nodes.values_mut().for_each(|x| x.init_bytes_to_string());
    }

    fn apply_tags(&mut self) {
        for node in self.nodes.values_mut() {
            node.apply_tags();
        }
    }

    fn init_loops(&mut self) {
        self.nodes.values_mut().for_each(|x| x.init_loops());
        if self.options.debug {
            self.nodes.values().for_each(|x| {
                if !x.call_loop.is_empty() {
                    log::info!("{:?} in loop with {:?}", x.name(), x.call_loop)
                }
            });
        }
    }

    fn init_node_connections(&mut self) {
        let lookup_edges = self.edges_up.clone();
        for node in self.nodes.values_mut() {
            if let Some(found_callers) = lookup_edges.get(&node.name()) {
                node.in_connection = found_callers.clone();
            }
        }
    }

    pub fn get_edges_with_memory_access(&self) -> Edges {
        self.edges_up
            .iter()
            .filter(|(x, _y)| match ***x {
                AsmWord::Address(_) => true,
                AsmWord::Symbol(_) => false,
            })
            .map(|(x, y)| (x.clone(), y.clone()))
            .collect()
    }

    fn init_edges(&mut self) {
        let lookup_nodes = self.nodes.clone();
        for node in lookup_nodes.values() {
            if node.out_connection.is_empty() {
                continue;
            } else {
                let callees = node.out_connection.clone();
                for callee in callees {
                    self.add_edge(&node.name(), &callee.clone());
                }
            }
        }
    }

    pub fn find_trees(&mut self) {
        todo!()
    }

    pub fn from_files<P: AsRef<Path>>(options: GraphOptions, files: Vec<P>) -> Self {
        let mut nodes: Nodes = BTreeMap::new();

        for i in files {
            let file = AsmParser::parse(i);
            for (word, symbol) in file.symbols {
                nodes.insert(word, GraphNode::from(symbol));
            }
        }

        Graph { options, nodes, ..Default::default() }
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Whatever> {
        let options = self.options;
        let dir = path.as_ref(); // Config Path
        let tree_dir = dir.join("by_tree");
        let graph_dir = dir.join("all_nodes");
        let addr_dir = dir.join("by_address");

        // TreePrinting
        for (name, tree) in &self.trees {
            // let level = format!("level_{}", tree.level);
            let name = name.to_string();
            let mut file = tree_dir.join(name);
            file.set_extension("txt");
            let file = create_file_and_dirs(file).expect("cannot create file");

            let mut writer = BufWriter::new(file);
        }

        Ok(())
    }
}
