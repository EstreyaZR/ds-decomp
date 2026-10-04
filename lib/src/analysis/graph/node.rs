use super::*;

#[derive(EnumString, Debug, strum_macros::Display, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[strum(ascii_case_insensitive)]
pub enum GraphFileType {
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

#[derive(Debug, strum_macros::Display, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum GraphNodeTag {
    #[allow(dead_code)]
    Address,
    File(GraphFileType),
    Collection,
}

pub struct AsmWordFromStrErr;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct GraphNodeTags(Vec<GraphNodeTag>);

impl GraphNodeTags {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GraphNodes(pub Vec<GraphNode>);

#[derive(Debug, strum_macros::Display, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum GraphNodeTreeType {
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

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GraphNode {
    pub name: Rc<AsmWord>,
    pub tag: GraphNodeTags,
    pub byte: Vec<u16>,
    pub byte_as_str: Vec<String>,
    pub word: WordVec,
    pub called_by: WordVec,
    pub call_loop: WordVec,
    pub tree_type: GraphNodeTreeType,
}

impl GraphNode {
    /// Takes care of actually tagging [GraphNode],
    /// please add calls for your tag check functions here,
    /// and your tags to [GraphNodeTag]
    pub fn apply_tags(&mut self) {
        if self.is_collection() {
            self.add_tag(GraphNodeTag::Collection);
        } else if self.is_address() {
            self.add_tag(GraphNodeTag::Address);
        } else if let Some(file_type) = self.is_file() {
            self.add_tag(GraphNodeTag::File(file_type));
        }
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
        let s = self.byte_as_str.join("");
        if let Some(s) = s.to_lowercase().split('.').next_back() {
            return GraphFileType::from_str(s).ok();
        }
        None
    }

    pub fn add_tag(&mut self, tag: GraphNodeTag) {
        self.tag.0.push(tag);
    }

    pub fn has_tree_type(&self, tree_type: GraphNodeTreeType) -> bool {
        self.tree_type == tree_type
    }

    pub fn remove_from_word(&mut self, word: &Rc<AsmWord>) {
        self.word = self.word.iter().filter(|x| !x.eq(&word)).cloned().collect();
    }

    pub fn remove_from_called_by(&mut self, word: &Rc<AsmWord>) {
        self.called_by = self.called_by.iter().filter(|x| !x.eq(&word)).cloned().collect();
    }

    pub fn init_in_loop_with(&mut self) {
        self.call_loop = self.word.iter().filter(|x| self.called_by.contains(x)).cloned().collect();
    }

    pub fn remove_loop_with_symbols_in_called_by(&mut self) {
        self.called_by =
            self.called_by.iter().filter(|x| !self.call_loop.contains(x)).cloned().collect();
    }

    pub fn remove_loop_with_symbols_in_word(&mut self) {
        self.word = self.word.iter().filter(|x| !self.call_loop.contains(x)).cloned().collect();
    }

    pub fn is_tree(&self) -> bool {
        match self.tree_type {
            GraphNodeTreeType::TreeRoot
            | GraphNodeTreeType::TreeMult
            | GraphNodeTreeType::TreeSimple
            | GraphNodeTreeType::TreeInit => true,
            _ => false,
        }
    }

    pub fn update_tree_type(&mut self) {
        self.tree_type = match (self.called_by.len(), self.word.len()) {
            (0, 0) => GraphNodeTreeType::Orphan,
            (1, 0) => GraphNodeTreeType::LeafSimple,
            (2.., 0) => GraphNodeTreeType::LeafMult,
            (0, 1..) => GraphNodeTreeType::Root,
            (1, 1..) => GraphNodeTreeType::NodeSimple,
            (2.., 1..) => GraphNodeTreeType::NodeMult,
        }
    }

impl GraphNodes {
    pub fn push(&mut self, item: GraphNode) {
        self.0.push(item);
    }
}

// STD-Trait Implementations

impl IntoIterator for GraphNodes {
    type IntoIter = IntoIter<Self::Item>;
    type Item = GraphNode;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl IntoIterator for GraphNodeTags {
    type IntoIter = IntoIter<Self::Item>;
    type Item = GraphNodeTag;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
