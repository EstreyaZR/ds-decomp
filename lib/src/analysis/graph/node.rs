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
    Pointer,
    File(GraphFileType),
    Collection,
}
#[derive(Debug, Clone)]
pub struct AsmWordFromStrErr;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct GraphNodeTags(Vec<GraphNodeTag>);

impl GraphNodeTags {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, strum_macros::Display, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum NodeTreeType {
    #[default]
    Init,
    Orphan,
    Leaf,
    Root,
    Node,
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GraphNode {
    pub symbol: AsmSymbol,
    pub tag: GraphNodeTags,
    pub out_connection: WordVec,
    pub in_connection: WordVec,
    pub call_loop: WordVec,
    pub tree_type: NodeTreeType,
    pub bytes_as_str: String,
}

impl GraphNode {
    /// Takes care of actually tagging [GraphNode],
    /// please add calls for your tag check functions here,
    /// and your tags to [GraphNodeTag]
    pub fn apply_tags(&mut self) {
        match self.symbol.asm_type {
            AsmSymbolType::Data => {
                self.check_is_file();
                self.check_is_collection_or_pointer();
            }
            AsmSymbolType::ArmFunction | AsmSymbolType::ThumbFunction => {}
            _ => {
                todo!()
            }
        }
    }

    pub fn name(&self) -> Rc<AsmWord> {
        self.symbol.name.clone()
    }

    pub fn check_is_collection_or_pointer(&mut self) {
        match (self.symbol.content.is_empty(), self.out_connection.len()) {
            (true, 2..) => {
                self.add_tag(GraphNodeTag::Collection);
            }
            (true, 1) => {
                self.add_tag(GraphNodeTag::Pointer);
            }
            _ => {}
        }
    }

    pub fn parse_build_info(&self) {
        todo!()
    }

    pub fn init_bytes_to_string(&mut self) {
        self.bytes_as_str =
            self.symbol.content.iter().map(|x| String::from_utf8(x.to_owned()).unwrap()).collect();
    }

    pub fn check_is_file(&mut self) {
        if let Some(s) = self.bytes_as_str.split('.').next_back() {
            if let Ok(filetype) = GraphFileType::from_str(s) {
                self.add_tag(GraphNodeTag::File(filetype));
            }
        }
    }

    pub fn add_tag(&mut self, tag: GraphNodeTag) {
        self.tag.0.push(tag);
    }

    //
    pub fn has_tree_type(&self, tree_type: NodeTreeType) -> bool {
        self.tree_type == tree_type
    }

    pub fn init_loops(&mut self) {
        self.call_loop = self
            .out_connection
            .iter()
            .filter(|x| self.in_connection.contains(x))
            .cloned()
            .collect();
    }

    pub fn calls_self(&self) -> bool {
        self.call_loop.contains(&self.name())
    }

    /// Removes calls to oneself, as well as any other loops from in_connection. Unused
    #[allow(dead_code)]
    pub fn resolve_loops(&mut self) {
        let selfname = self.name();
        self.in_connection = self
            .in_connection
            .iter()
            .filter(|x| !x.eq(&&selfname) && !self.call_loop.contains(x.to_owned()))
            .map(|x| x.to_owned())
            .collect();
    }

    pub fn init_tree_type(&mut self) {
        match self.symbol.asm_type {
            AsmSymbolType::Bss(_) => self.tree_type = NodeTreeType::Leaf,
            _ => {
                self.update_tree_type();
            }
        }
    }

    pub fn update_tree_type(&mut self) {
        self.tree_type = match (self.in_connection.len(), self.out_connection.len()) {
            (0, 0) => NodeTreeType::Orphan,
            (0, 1..) => NodeTreeType::Root,
            (1.., 0) => NodeTreeType::Leaf,
            (1.., 1..) => NodeTreeType::Node,
        }
    }
}

impl From<AsmSymbol> for GraphNode {
    fn from(value: AsmSymbol) -> Self {
        let out_connection = value.outgoing.iter().map(|x| Rc::new(x.to_owned())).collect();
        GraphNode { symbol: value, out_connection, ..Default::default() }
    }
}

impl IntoIterator for GraphNodeTags {
    type IntoIter = IntoIter<Self::Item>;
    type Item = GraphNodeTag;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
