mod tree;
pub(crate) use tree::*;
mod asm;
pub(crate) use asm::*;
mod node;
use std::{
    collections::BTreeMap,
    fmt::Display,
    io::{BufWriter, Write},
    path::Path,
    rc::Rc,
    str::FromStr,
    vec::IntoIter,
};

pub(crate) use node::*;
use snafu::Whatever;
use strum::EnumString;

use crate::util::io::*;

type WordVec = Vec<Rc<GraphNodeWord>>;
type Trees = BTreeMap<Rc<GraphNodeWord>, Tree>;
type Nodes = BTreeMap<Rc<GraphNodeWord>, GraphNode>;
type Edges = BTreeMap<Rc<GraphNodeWord>, WordVec>;

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

use crate::util::{io::read_to_string, parse::parse_u16};
#[derive(Default)]
pub struct Graph {
    options: GraphOptions,
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
        self.nodes.insert(node.name.clone(), node);
    }

    fn pop_node(&mut self, node: Self::Indexer) -> Option<Self::Item> {
        self.nodes.remove(&node)
    }
}

impl Graph {
    pub fn init_nodes(&mut self) {
        self.clean_byte_strings();
        log::info!("Finished CleanByteStrings");

        self.init_edges();
        log::info!("Finished InitEdges");

        self.init_called_by();
        log::info!("Finished InitCalledBy");

        self.init_loops();
        log::info!("Finished InitLoops");

        self.resolve_loops();

        self.apply_tags();
        log::info!("Finished ApplyTags");

        self.init_tree_status();
        log::info!("Finished InitTreeStatus");
    }

    fn resolve_loops(&mut self) {
        self.nodes.values_mut().for_each(|x| x.remove_loop_with_symbols_in_word());
    }

    fn apply_tags(&mut self) {
        for node in self.nodes.values_mut() {
            node.apply_tags();
        }
    }

    fn clean_byte_strings(&mut self) {
        for node in self.nodes.values_mut() {
            node.clean();
        }
    }

    fn init_loops(&mut self) {
        self.nodes.values_mut().for_each(|x| x.init_in_loop_with());
        if self.options.debug {
            self.nodes.values().for_each(|x| {
                if !x.call_loop.is_empty() {
                    log::info!("{:?} in loop with {:?}", x.name, x.call_loop)
                }
            });
        }
    }

    fn init_called_by(&mut self) {
        let lookup_edges = self.edges_up.clone();
        for node in self.nodes.values_mut() {
            if let Some(found_callers) = lookup_edges.get(&node.name) {
                node.called_by = found_callers.clone();
            }
        }
    }

    fn init_edges(&mut self) {
        let lookup_nodes = self.nodes.clone();
        for node in lookup_nodes.values() {
            if node.word.is_empty() {
                continue;
            } else {
                let callees = node.word.clone();
                for callee in callees {
                    self.add_edge(&node.name.clone(), &callee.clone());
                }
            }
        }
    }

    fn init_tree_status(&mut self) {
        self.nodes.values_mut().for_each(|node| {
            if node.tree_type == GraphNodeTreeType::TreeMult
                || node.tree_type == GraphNodeTreeType::TreeInit
            {
                node.tree_type = match node.called_by.len() {
                    0 => GraphNodeTreeType::TreeRoot,
                    1 => GraphNodeTreeType::TreeSimple,
                    2.. => GraphNodeTreeType::TreeMult,
                }
            } else {
                node.update_tree_type();
            }
        });
    }

    pub fn find_trees(&mut self, iteration: u8) {
        let nodes_in_graph = self.nodes.len();
        let edges = self.edges_down.clone();

        for (node, vector) in edges.iter() {
            let simple_nodes: Vec<_> = vector
                .iter()
                .filter(|x| {
                    if let Some(real_node) = self.nodes.get(x.to_owned()) {
                        real_node.has_tree_type(GraphNodeTreeType::LeafSimple)
                            || real_node.has_tree_type(GraphNodeTreeType::TreeSimple)
                    } else {
                        false
                    }
                })
                .collect();
            if !simple_nodes.is_empty()
                && let Some(tree_node) = self.pop_node(node.to_owned())
            {
                let mut tree_node = tree_node.clone();
                let mut tree = Tree::new_from_node(tree_node.clone());

                tree.level = iteration;

                for child in vector {
                    if let Some(found_child) = self.pop_node(child.to_owned()) {
                        match found_child.tree_type {
                            GraphNodeTreeType::TreeSimple => {
                                if let Some(popped_tree) = self.trees.remove(&found_child.name) {
                                    tree.consume_tree(popped_tree);
                                }
                                tree_node.remove_from_word(&found_child.name);
                            }
                            GraphNodeTreeType::LeafSimple => {
                                tree.add_node(found_child.clone());
                                tree_node.remove_from_word(&found_child.name);
                            }
                            _ => {
                                self.add_node(found_child);
                            }
                        }
                    }
                }

                tree_node.tree_type = GraphNodeTreeType::TreeInit;
                self.add_node(tree_node);
                self.trees.insert(tree.get_root(), tree);
            }
        }

        let nodes_in_graph_new = self.nodes.len();

        if self.options.debug {
            log::info!(
                "{} Nodes remain from {} scanned in Source Assembly\nGraph has {} Trees",
                nodes_in_graph_new,
                nodes_in_graph,
                self.trees.len()
            );
        }
        self.init_tree_status();
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

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Whatever> {
        let dir = path.as_ref(); // Config Path
        let tree_dir = dir.join("tree");

        // TreePrinting
        for (name, tree) in &self.trees {
            let level = format!("level_{}", tree.level);
            let name = name.to_string();
            let mut file = tree_dir.join(level);
            file.push(name);
            file.set_extension("txt");
            let file = create_file_and_dirs(file).expect("cannot create file");

            let mut writer = BufWriter::new(file);

            let _ = writer.write_fmt(format_args!("{}", tree.print_file()));
        }
        // Remaining Nodes in Graph Printing

        Ok(())
    }
}
