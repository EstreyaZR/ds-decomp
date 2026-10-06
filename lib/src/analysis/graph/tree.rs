use super::*;

#[allow(unused)]
pub trait TreeTrait: Eq + Ord {
    // fn new(name: Rc<AsmWord>) -> Self;
    fn new_from_node(node: GraphNode) -> Self;
    fn get_root(&self) -> Rc<AsmWord>;
    // fn set_root(&mut self, node: Rc<AsmWord>) -> Rc<AsmWord>;
    fn add_node(&mut self, node: GraphNode) -> Rc<AsmWord>;
    fn consume_tree(&mut self, tree: Self);
    // fn consume_node(&mut self, node: GraphNode) -> Rc<AsmWord>;
    // fn pop_node(&mut self, node: Rc<AsmWord>) -> Option<GraphNode>;
}

#[derive(Default, Debug, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub struct Tree {
    pub root: GraphNode,
    pub nodes: Nodes,
    pub level: u8,
}

impl TreeTrait for Tree {
    fn new_from_node(node: GraphNode) -> Self {
        Self { root: node.clone(), ..Default::default() }
    }

    fn add_node(&mut self, node: GraphNode) -> Rc<AsmWord> {
        let name = node.name();
        self.nodes.insert(name.clone(), node);
        name
    }

    fn get_root(&self) -> Rc<AsmWord> {
        self.root.name()
    }

    fn consume_tree(&mut self, tree: Self) {
        let mut old_root = tree.root;
        old_root.update_tree_type();

        self.add_node(old_root);

        for node in tree.nodes.values().to_owned() {
            self.add_node(node.clone());
        }
    }
}

impl Tree {
    pub fn print_file(&self) -> String {
        let mut s = String::new();
        let main_node = self.get_root().to_string();
        s.push_str(format!("[{}]", main_node).as_str());
        for name in self.nodes.keys() {
            s.push('\n');
            s.push_str(format!("  -> {}", name).as_str());
        }
        s.push_str("\n\n> Definitions Start <\n\n");
        // s.push_str(&self.root.print_file());
        for node in self.nodes.values() {
            s.push('\n');
            // s.push_str(&node.print_file());
        }
        s
    }
}
