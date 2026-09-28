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
    Address,
    File(GraphFileType),
    Collection,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum GraphNodeWord {
    Address(u32),
    Symbol(String),
}

pub struct GraphNodeWordFromStrErr;

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
    pub name: Rc<GraphNodeWord>,
    pub tag: GraphNodeTags,
    pub byte: Vec<u16>,
    pub byte_as_str: Vec<String>,
    pub word: WordVec,
    pub called_by: WordVec,
    pub call_loop: WordVec,
    pub tree_type: GraphNodeTreeType,
}

impl GraphNode {
    pub fn from_entry(entry: AsmEntry) -> Self {
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

    pub fn consume_entry(self, entry: AsmEntry) -> Self {
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
    pub fn clean(&mut self) {
        self.byte_as_str = self
            .byte_as_str
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| x.trim_matches([b' ' as char, '\0']).to_string())
            .collect::<Vec<String>>();

        self.word.sort_unstable();
        self.word.dedup();
    }

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

    #[allow(dead_code)]
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

    pub fn remove_from_word(&mut self, word: &Rc<GraphNodeWord>) {
        self.word = self.word.iter().filter(|x| !x.eq(&word)).cloned().collect();
    }

    pub fn remove_from_called_by(&mut self, word: &Rc<GraphNodeWord>) {
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

    pub fn print_file(&self) -> String {
        let mut s = String::new();
        s.push_str(format!("[{}]\t{}\n", self.name, self.tree_type).as_str());
        if !self.tag.is_empty() {
            s.push_str("\t[Tags]");
            for i in self.tag.clone().into_iter() {
                s.push('\t');
                s.push_str(&i.to_string());
            }
            s.push('\n');
        }

        if !self.byte.is_empty() {
            let remainder = self.byte_size() - 1;
            let full_rows = self.byte_size() - remainder - 1;
            for i in 0..full_rows {
                s.push('\n');
                s.push('\t');
                for j in 0..7 {
                    s.push_str(format!("{:x} ", self.byte[i * 8 + j]).as_str());
                }
            }
            s.push('\n');
            s.push('\t');
            for k in 0..remainder {
                s.push_str(format!("{:x}", self.byte[full_rows * 8 + k]).as_str());
            }
            s.push('\n');
            s.push_str("\tString:");
            s.push_str(&self.byte_as_str.join(""));
            s.push('\n');
        }

        if !self.word.is_empty() {
            s.push('\n');
            s.push_str("\t[Word]");
            for i in self.word.iter() {
                s.push('\n');
                s.push('\t');
                s.push_str(&i.to_string());
            }
            s.push('\n');
        }

        if !self.called_by.is_empty() {
            s.push('\n');
            s.push_str("\t[CalledBy]");
            for i in self.called_by.iter() {
                s.push('\n');
                s.push('\t');
                s.push_str(&i.to_string());
            }
            s.push('\n');
        }

        if !self.call_loop.is_empty() {
            s.push('\n');
            s.push_str("\t[CallLoop]");
            for i in self.call_loop.iter() {
                s.push('\n');
                s.push('\t');
                s.push_str(&i.to_string());
            }
            s.push('\n');
        }

        s
    }
}

impl GraphNodes {
    pub fn push(&mut self, item: GraphNode) {
        self.0.push(item);
    }
}

// STD-Trait Implementations

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
