#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(deprecated)]
use std::ffi::OsString;

use super::*;
use crate::util::parse::parse_u8;

#[derive(Debug)]
pub struct AsmFile {
    pub name: OsString,
    pub syntax_global: bool,
    pub path: std::path::PathBuf,
    pub symbols: BTreeMap<Rc<AsmWord>, AsmSymbol>,
    pub outgoing_symbols: Vec<Rc<AsmWord>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AsmWord {
    Address(u32),
    Symbol(String),
}

impl Display for AsmWord {
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

impl Default for AsmWord {
    fn default() -> Self {
        Self::Symbol(String::default())
    }
}

impl FromStr for AsmWord {
    type Err = AsmWordFromStrErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_string().into())
    }
}

impl From<String> for AsmWord {
    fn from(value: String) -> Self {
        match value.strip_prefix("0x") {
            Some(x) => Self::Address(u32::from_str_radix(x, 16).unwrap()),
            None => Self::Symbol(value),
        }
    }
}

impl AsmWord {
    pub fn get_address(&self) -> Option<u32> {
        match self {
            AsmWord::Address(addr) => Some(*addr),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AsmSectionType {
    Text,
    Data,
    Rodata,
    Bss,
    Custom(String),

    Unknown,
}

#[derive(Debug, Default)]
pub struct AsmParser;
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AsmType {
    Syntax,
    Include,
    Global,
    Section(AsmSectionType),

    ArmFunctionStart,
    ArmFunctionEnd,

    ThumbFunctionStart,
    ThumbFunctionEnd,

    Label,
    Symbol,
    Byte,
    Word,

    Jump,
    CoProcessor,

    #[default]
    Unknown,
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AsmSymbolType {
    ArmFunction,
    ThumbFunction,
    Data,
    Bss(u32),
    #[default]
    Unknown,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AsmLine {
    asm_type: AsmType,
    content: Vec<String>,
}
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AsmLines(Vec<AsmLine>);

impl AsmLines {
    pub fn collect_to_symbol(&mut self) -> AsmSymbol {
        let mut global = false;
        let mut name = AsmWord::default();
        let mut asm_type = AsmSymbolType::default();
        let mut outgoing: Vec<AsmWord> = Vec::new();
        let mut content: Vec<Vec<u8>> = Vec::new();

        for line in self.0.drain(..) {
            match line.asm_type {
                AsmType::Symbol => {
                    name = line.get_name_from_symbol();
                    if let Some(size) = line.parse_space_from_symbol() {
                        asm_type = AsmSymbolType::Bss(size);
                    }
                }
                AsmType::ArmFunctionStart => {
                    asm_type = AsmSymbolType::ArmFunction;
                }
                AsmType::ThumbFunctionStart => {
                    asm_type = AsmSymbolType::ArmFunction;
                }
                AsmType::ArmFunctionEnd | AsmType::ThumbFunctionEnd => {
                    break;
                }
                AsmType::Global => {
                    global = true;
                }
                AsmType::Label => {
                    if let Some(word) = line.parse_word_from_label() {
                        outgoing.push(word);
                    }
                }
                AsmType::Byte => content.push(line.get_byte_as_vec()),
                _ => {
                    todo!()
                }
            }
        }

        AsmSymbol { global, name: Rc::new(name), asm_type, outgoing, content }
    }

    pub fn push(&mut self, line: &AsmLine) {
        self.0.push(line.to_owned());
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AsmSymbol {
    pub name: Rc<AsmWord>,
    pub global: bool,
    pub asm_type: AsmSymbolType,
    pub outgoing: Vec<AsmWord>,
    pub content: Vec<Vec<u8>>,
}

impl AsmLine {
    pub fn parse(source: &str) -> Self {
        let source = source.to_owned();
        let source: Vec<_> = source.split_whitespace().collect();
        let mut asm_type: AsmType = match source[0] {
            s if s.starts_with(".byte") => AsmType::Byte,
            s if s.starts_with(".word") => AsmType::Word,
            //s if s.starts_with(".space") => AsmType::Space,
            s if s.starts_with(".syntax") => AsmType::Syntax,
            s if s.starts_with(".include") => AsmType::Include,
            s if s.starts_with(".global") => AsmType::Global,
            s if s.starts_with(".text") => AsmType::Section(AsmSectionType::Text),
            s if s.starts_with(".section") => AsmType::Section(AsmSectionType::Unknown),

            s if s.starts_with("arm_func_start") => AsmType::ArmFunctionStart,
            s if s.starts_with("arm_func_end") => AsmType::ArmFunctionEnd,
            s if s.starts_with("thumb_func_start") => AsmType::ThumbFunctionStart,
            s if s.starts_with("thumb_func_end") => AsmType::ThumbFunctionEnd,

            s if !s.starts_with(".L_") && s.ends_with(":") => AsmType::Symbol,
            s if s.starts_with(".L_") => AsmType::Label,

            _ => AsmType::default(),
        };

        if asm_type == AsmType::Section(AsmSectionType::Unknown) {
            asm_type = match source[1] {
                s if s.contains(".data") => AsmType::Section(AsmSectionType::Data),
                s if s.contains(".rodata") => AsmType::Section(AsmSectionType::Rodata),
                s if s.contains(".bss") => AsmType::Section(AsmSectionType::Bss),
                s => AsmType::Section(AsmSectionType::Custom(s.to_string())),
            }
        }

        Self { asm_type, content: source.iter().map(|v| v.to_string()).collect() }
    }

    pub fn parse_word_from_label(&self) -> Option<AsmWord> {
        if self.content.len() == 3 {
            if self.content[1].contains(".word") {
                let word = AsmWord::from(self.content[2].clone());
                return Some(word);
            }
        }
        None
    }

    pub fn parse_space_from_symbol(&self) -> Option<u32> {
        if self.content.len() == 3 {
            if self.content[1].contains(".space") {
                let size = u32::from_str_radix(self.content[2].as_str(), 16).unwrap();
                return Some(size);
            }
        }
        None
    }

    pub fn get_name_from_symbol(&self) -> AsmWord {
        let mut name = self.content[1].clone();
        AsmWord::from(name.trim_end_matches(':').to_string())
    }

    pub fn get_byte_as_vec(&self) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        for num in &self.content[1..] {
            v.push(parse_u8(num).unwrap());
        }
        v
    }
}

impl AsmParser {
    pub fn parse<P: AsRef<Path>>(file: P) -> AsmFile {
        let file = file.as_ref();
        let buffer = read_to_string(file).unwrap();
        let lines = buffer.lines();

        // filter everything except definitions, words and bytes
        let lines: Vec<_> = lines.map(|line| AsmLine::parse(line.trim_start())).collect();

        let mut symbols = BTreeMap::new();
        let mut asm_lines: AsmLines = AsmLines::default();
        let mut in_function_switch = false;
        let mut section: AsmSectionType = AsmSectionType::Unknown;
        let mut syntax_global = false;

        for line in lines.iter() {
            match line.asm_type {
                // Headers
                AsmType::Section(ref sec) => {
                    section = sec.to_owned();
                }
                AsmType::Syntax => {
                    syntax_global = true;
                }

                // Function-Related Section
                AsmType::ArmFunctionStart | AsmType::ThumbFunctionStart | AsmType::Global => {
                    if !asm_lines.0.is_empty() && in_function_switch == false {
                        let symbol = asm_lines.collect_to_symbol();
                        symbols.insert(symbol.name.clone(), symbol);
                    }
                    asm_lines.push(line);
                    in_function_switch = true;
                }

                AsmType::Word | AsmType::Label => {
                    asm_lines.push(line);
                }

                AsmType::Symbol => {
                    if in_function_switch {
                        asm_lines.push(line);
                    } else {
                        let symbol = asm_lines.collect_to_symbol();
                        symbols.insert(symbol.name.clone(), symbol);
                        asm_lines.push(line);
                    }
                }

                AsmType::ArmFunctionEnd | AsmType::ThumbFunctionEnd => {
                    if section == AsmSectionType::Text && in_function_switch {
                        in_function_switch = false;
                        asm_lines.push(line);
                        let symbol = asm_lines.collect_to_symbol();
                        symbols.insert(symbol.name.clone(), symbol);
                    } else {
                        panic!("ArmFunction Declaration found outside of Text Section");
                    }
                }
                // Function Section End
                // Data / Rodata Section

                // Data / Rodata Section End
                _ => continue,
            }
        }

        // Push the last symbol in line, if the vec is not empty
        // Especially Important for non Function Containing Files
        if !lines.is_empty() {
            let symbol = asm_lines.collect_to_symbol();
            symbols.insert(symbol.name.clone(), symbol);
        }
        let mut outgoing_dirty: Vec<AsmWord> =
            symbols.values().flat_map(|x| x.outgoing.clone()).collect();
        outgoing_dirty.sort_unstable();
        outgoing_dirty.dedup();

        let declared_in_file: Vec<_> = symbols.keys().cloned().collect();

        let mut outgoing_symbols: Vec<_> = outgoing_dirty
            .iter()
            .map(|x| Rc::new(x.to_owned()))
            .filter(|word| !declared_in_file.contains(word))
            .collect();

        AsmFile {
            name: file.file_name().unwrap().to_owned(),
            path: file.to_path_buf(),
            symbols,
            outgoing_symbols,
            syntax_global,
        }
    }
}

impl IntoIterator for AsmLines {
    type IntoIter = IntoIter<Self::Item>;
    type Item = AsmLine;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
