use super::*;

#[derive(Debug, Default)]
pub struct AsmParser;

/// Simple Vector Collection for [AsmEntry]
#[derive(Default, Debug, Clone)]
pub struct AsmEntries(Vec<AsmEntry>);

/// Growable entry of at least one parsed line of Assembly.
#[derive(Default, Clone, Debug)]
pub struct AsmEntry {
    pub name: Option<String>,
    pub label: bool,
    #[allow(dead_code)]
    pub bss: bool,
    pub byte: Option<Vec<u16>>,
    pub byte_as_str: Option<String>,
    pub word: Option<String>,
}

impl AsmEntry {
    /// Doesn't actually assume to fail from input
    pub fn from_str(source: &str) -> Result<Option<Self>, Whatever> {
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
                "r0," | "r1," | "r2," | "r3," | "r4," | "r5," | "r6," | "r7," | "r8," | "r9,"
                | "r10," | "r11," | "lr," | "ip," | "pc," => None,
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

    /// Helper func to detect Files.
    pub fn byte_as_str(&self) -> String {
        if let Some(byte_as_str) = &self.byte_as_str {
            byte_as_str.to_string()
        } else {
            String::new()
        }
    }

    pub fn byte(&self) -> Vec<u16> {
        if let Some(byte) = &self.byte { byte.to_owned() } else { Vec::new() }
    }

    pub fn word_as_graph_node_word(&self) -> Option<GraphNodeWord> {
        self.word.as_ref().map(|word| word.to_string().into())
    }
}

impl AsmEntries {
    fn add(&mut self, x: AsmEntry) {
        self.0.push(x)
    }

    pub fn to_nodes(&self) -> GraphNodes {
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

/// Parses a ASM File line-by-line to [AsmEntry], creating the collection [AsmEntries]
impl AsmParser {
    pub fn parse<P: AsRef<Path>>(file: P) -> Result<AsmEntries, Whatever> {
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

// STD-Trait Implementations

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

impl IntoIterator for AsmEntries {
    type IntoIter = IntoIter<Self::Item>;
    type Item = AsmEntry;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
