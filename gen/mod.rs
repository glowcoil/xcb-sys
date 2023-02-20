use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::str::FromStr;

use roxmltree::Document;

fn sanitize(name: &str) -> &str {
    match name {
        "type" => "type_",
        "match" => "match_",
        name => name,
    }
}

fn convert_type_name(extension_name: Option<&str>, name: &str) -> String {
    let mut out = String::new();

    out.push_str("xcb_");

    if let Some(extension_name) = extension_name {
        out.push_str(&convert_extension_name(extension_name));
        out.push('_');
    }

    out.push_str(&convert_name(name));

    out.push_str("_t");

    out
}

fn convert_enum_item_name(
    extension_name: Option<&str>,
    enum_name: &str,
    item_name: &str,
) -> String {
    let mut out = String::new();

    out.push_str("xcb_");

    if let Some(extension_name) = extension_name {
        out.push_str(&convert_extension_name(extension_name));
        out.push('_');
    }

    out.push_str(&convert_name(enum_name));
    out.push('_');

    out.push_str(&convert_name(item_name));

    out.to_uppercase()
}

fn convert_extension_name(name: &str) -> String {
    match name {
        "XPrint" | "XCMisc" | "BigRequests" => name.to_lowercase(),
        _ => convert_name(name),
    }
}

fn convert_name(name: &str) -> String {
    // Replicates the name conversion logic in libxcb's generator script[0]. Names are split using
    // the regular expression
    //
    //    ([A-Z0-9][a-z]+|[A-Z0-9]+(?![a-z])|[a-z]+)
    //
    // and then joined with underscores. The (?!...) notation is for negative lookahead, i.e.
    // a(?!b) matches "a" if it is not followed by "b"[1].
    //
    // [0]: https://gitlab.freedesktop.org/xorg/lib/libxcb/-/blob/fd04ab24a5e99d53874789439d3ffb0eb82574f7/src/c_client.py
    // [1]: https://docs.python.org/3/library/re.html#regular-expression-syntax

    if name == "DECnet" {
        return String::from("decnet");
    }

    let mut out = String::new();
    let mut first = true;
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            // [A-Z0-9][a-z]+
            if chars.peek().map_or(false, |next| next.is_ascii_lowercase()) {
                if !first {
                    out.push('_');
                }
                first = false;

                out.extend(c.to_lowercase());

                while let Some(c) = chars.next_if(|c| c.is_ascii_lowercase()) {
                    out.extend(c.to_lowercase());
                }
            // [A-Z0-9]+(?![a-z])
            } else {
                if !first {
                    out.push('_');
                }
                first = false;

                out.extend(c.to_lowercase());

                while let Some(&next) = chars.peek() {
                    if !(next.is_ascii_uppercase() || next.is_ascii_digit()) {
                        break;
                    }

                    let mut lookahead = chars.clone();
                    lookahead.next();
                    if let Some(next_next) = lookahead.next() {
                        if next_next.is_ascii_lowercase() {
                            break;
                        }
                    }

                    out.extend(next.to_lowercase());
                    chars.next();
                }
            }
        // [a-z]+
        } else if c.is_ascii_lowercase() {
            if !first {
                out.push('_');
            }
            first = false;

            out.extend(c.to_lowercase());

            while let Some(c) = chars.next_if(|c| c.is_ascii_lowercase()) {
                out.extend(c.to_lowercase());
            }
        } else {
            continue;
        }
    }

    out
}

#[derive(Debug)]
struct Ast {
    global_types: BTreeMap<&'static str, &'static str>,
    modules: BTreeMap<String, Module>,
}

impl Ast {
    fn lookup(&self, header: &str, type_name: &str) -> Option<&str> {
        if let Some(colon) = type_name.find(':') {
            let module_name = &type_name[0..colon];
            let name = &type_name[colon + 1..];

            return self.modules[module_name].types.get(name).map(|t| &*t.name);
        }

        if let Some(result) = self.lookup_inner(header, type_name) {
            Some(result)
        } else {
            self.global_types.get(&*type_name).copied()
        }
    }

    fn lookup_inner(&self, header: &str, type_name: &str) -> Option<&str> {
        let module = &self.modules[header];

        if let Some(type_) = module.types.get(type_name) {
            return Some(&type_.name);
        }

        for import in &module.imports {
            if let Some(result) = self.lookup_inner(import, type_name) {
                return Some(result);
            }
        }

        None
    }
}

#[derive(Debug)]
struct Module {
    extension_name: Option<String>,
    major_version: Option<String>,
    minor_version: Option<String>,
    imports: Vec<String>,
    types: BTreeMap<String, Type>,
}

#[derive(Debug)]
struct Type {
    name: String,
    kind: Kind,
}

#[derive(Debug)]
enum Kind {
    Id,
    Enum { items: Vec<(String, u32)> },
    TypeDef { value: String },
    Struct { fields: Vec<Field> },
    Union { fields: Vec<Field> },
}

#[derive(Debug)]
struct Field {
    name: String,
    type_: String,
}

pub fn gen(headers: &[&str], out_path: &Path) {
    let global_types = BTreeMap::from([
        ("CARD8", "u8"),
        ("CARD16", "u16"),
        ("CARD32", "u32"),
        ("CARD64", "u64"),
        ("INT8", "i8"),
        ("INT16", "i16"),
        ("INT32", "i32"),
        ("INT64", "i64"),
        ("BYTE", "u8"),
        ("BOOL", "u8"),
        ("char", "std::ffi::c_char"),
        ("float", "f32"),
        ("double", "f64"),
        ("void", "std::ffi::c_void"),
    ]);

    let mut modules = BTreeMap::new();

    for &header in headers {
        let mut path = Path::new("xml").join(header);
        path.set_extension("xml");

        let bytes = fs::read(path).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let tree = Document::parse(text).unwrap();
        let root = tree.root_element();

        if !root.has_tag_name("xcb") {
            panic!();
        }

        let header_name = header.to_string();
        let extension_name = root.attribute("extension-name").map(|s| s.to_lowercase());
        let major_version = root.attribute("major-version").map(|s| s.to_string());
        let minor_version = root.attribute("minor-version").map(|s| s.to_string());

        let mut imports = Vec::new();
        let mut types = BTreeMap::new();

        for child in root.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "import" => {
                        imports.push(child.text().unwrap().to_string());
                    }
                    "xidtype" | "xidunion" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let type_name =
                            convert_type_name(extension_name.as_ref().map(|s| &**s), &name);
                        types.insert(
                            name,
                            Type {
                                name: type_name,
                                kind: Kind::Id,
                            },
                        );
                    }
                    "enum" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let type_name =
                            convert_type_name(extension_name.as_ref().map(|s| &**s), &name);

                        let mut items = Vec::new();
                        for child in child.children() {
                            if child.is_element() {
                                if child.tag_name().name() == "item" {
                                    let item_name = child.attribute("name").unwrap();
                                    let full_item_name = convert_enum_item_name(
                                        extension_name.as_ref().map(|s| &**s),
                                        &name,
                                        item_name,
                                    );

                                    let choice = child.first_element_child().unwrap();
                                    let value = match choice.tag_name().name() {
                                        "value" => u32::from_str(choice.text().unwrap()).unwrap(),
                                        "bit" => {
                                            1 << u32::from_str(choice.text().unwrap()).unwrap()
                                        }
                                        _ => panic!(),
                                    };

                                    items.push((full_item_name, value));
                                }
                            }
                        }

                        types.insert(
                            name,
                            Type {
                                name: type_name,
                                kind: Kind::Enum { items },
                            },
                        );
                    }
                    "typedef" => {
                        let name = child.attribute("newname").unwrap().to_string();
                        let type_name =
                            convert_type_name(extension_name.as_ref().map(|s| &**s), &name);
                        let value = child.attribute("oldname").unwrap().to_string();
                        types.insert(
                            name,
                            Type {
                                name: type_name,
                                kind: Kind::TypeDef { value },
                            },
                        );
                    }
                    "struct" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let type_name =
                            convert_type_name(extension_name.as_ref().map(|s| &**s), &name);

                        let mut fields = Vec::new();
                        for child in child.children() {
                            if child.is_element() {
                                match child.tag_name().name() {
                                    "field" => {
                                        fields.push(Field {
                                            name: sanitize(child.attribute("name").unwrap())
                                                .to_string(),
                                            type_: child.attribute("type").unwrap().to_string(),
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }

                        types.insert(
                            name,
                            Type {
                                name: type_name,
                                kind: Kind::Struct { fields },
                            },
                        );
                    }
                    "union" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let type_name =
                            convert_type_name(extension_name.as_ref().map(|s| &**s), &name);

                        let mut fields = Vec::new();
                        for child in child.children() {
                            if child.is_element() {
                                match child.tag_name().name() {
                                    "field" => {
                                        fields.push(Field {
                                            name: sanitize(child.attribute("name").unwrap())
                                                .to_string(),
                                            type_: child.attribute("type").unwrap().to_string(),
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }

                        types.insert(
                            name,
                            Type {
                                name: type_name,
                                kind: Kind::Union { fields },
                            },
                        );
                    }
                    _ => {}
                }
            }
        }

        modules.insert(
            header_name,
            Module {
                extension_name,
                major_version,
                minor_version,
                imports,
                types,
            },
        );
    }

    let ast = Ast {
        global_types,
        modules,
    };

    let mut writer = BufWriter::new(File::create(out_path).unwrap());
    for (header_name, module) in &ast.modules {
        writeln!(writer, "pub mod {header_name} {{").unwrap();

        writeln!(writer, "    use super::*;").unwrap();

        for import in &module.imports {
            writeln!(writer, "    use super::{import}::*;").unwrap();
        }

        if let Some(extension_name) = &module.extension_name {
            let extension_name_uppercase = extension_name.to_uppercase();

            if let Some(major_version) = &module.major_version {
                writeln!(
                    writer,
                    "    pub const XCB_{extension_name_uppercase}_MAJOR_VERSION: u32 = {major_version};"
                )
                .unwrap();
            }

            if let Some(minor_version) = &module.minor_version {
                writeln!(
                    writer,
                    "    pub const XCB_{extension_name_uppercase}_MINOR_VERSION: u32 = {minor_version};"
                )
                .unwrap();
            }

            let ext = convert_extension_name(extension_name);
            writeln!(writer, "    extern \"C\" {{").unwrap();
            writeln!(writer, "        pub static xcb_{ext}_id: xcb_extension_t;").unwrap();
            writeln!(writer, "    }}").unwrap();
        }

        let mut id_names = BTreeSet::new();
        for type_ in module.types.values() {
            if let Kind::Id = &type_.kind {
                id_names.insert(&type_.name);
            }
        }

        for type_ in module.types.values() {
            let type_name = &type_.name;

            match &type_.kind {
                Kind::Id => {
                    writeln!(writer, "    pub type {type_name} = u32;").unwrap();
                }
                Kind::Enum { items } => {
                    // Some source files contain duplicate xidtype and enum declarations, so don't output an enum type
                    // alias if there's already one from the xidtype.
                    if !id_names.contains(type_name) {
                        writeln!(writer, "    pub type {type_name} = u32;").unwrap();
                    }
                    for (name, value) in items {
                        writeln!(writer, "    pub const {name}: {type_name} = {value};").unwrap();
                    }
                }
                Kind::TypeDef { value } => {
                    let field_type = ast
                        .lookup(header_name, &value)
                        .unwrap_or_else(|| panic!("{}", value));
                    writeln!(writer, "    pub type {type_name} = {field_type};").unwrap();
                }
                Kind::Struct { fields } => {
                    writeln!(writer, "    #[repr(C)]").unwrap();
                    writeln!(writer, "    #[derive(Copy, Clone)]").unwrap();
                    writeln!(writer, "    pub struct {type_name} {{").unwrap();

                    for field in fields {
                        let field_name = &field.name;
                        let field_type = ast
                            .lookup(header_name, &field.type_)
                            .unwrap_or_else(|| panic!("{}", field.type_));
                        writeln!(writer, "        pub {field_name}: {field_type},").unwrap();
                    }

                    writeln!(writer, "    }}").unwrap();
                }
                Kind::Union { fields } => {
                    writeln!(writer, "    #[repr(C)]").unwrap();
                    writeln!(writer, "    #[derive(Copy, Clone)]").unwrap();
                    writeln!(writer, "    pub union {type_name} {{").unwrap();

                    for field in fields {
                        let field_name = &field.name;
                        let field_type = ast
                            .lookup(header_name, &field.type_)
                            .unwrap_or_else(|| panic!("{}", field.type_));
                        writeln!(writer, "        pub {field_name}: {field_type},").unwrap();
                    }

                    // Temporary hack since empty unions don't build and we don't handle list fields yet.
                    writeln!(writer, "        _data: (),").unwrap();

                    writeln!(writer, "    }}").unwrap();
                }
            }
        }

        writeln!(writer, "}}").unwrap();
    }
}
