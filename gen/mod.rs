use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::str::FromStr;

use roxmltree::{Document, Node};

fn join(pieces: &[&str]) -> String {
    let mut iter = pieces.iter();

    let first = if let Some(first) = iter.next() {
        first
    } else {
        return String::new();
    };

    let mut out = String::new();
    out.push_str(first);

    for piece in iter {
        out.push('_');
        out.push_str(piece);
    }

    out
}

fn sanitize(name: &str) -> &str {
    match name {
        "type" => "type_",
        "match" => "match_",
        name => name,
    }
}

fn convert_extension_name(name: &str) -> String {
    match name {
        "XPrint" | "XCMisc" | "BigRequests" => convert_name(name),
        _ => name.to_lowercase(),
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
    prefix: String,
    major_version: Option<String>,
    minor_version: Option<String>,
    imports: Vec<String>,
    types: BTreeMap<String, Type>,
    requests: Vec<Request>,
    events: Vec<Event>,
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
    type_: FieldType,
}

#[derive(Debug)]
enum FieldType {
    Name(String),
    Padding(u32),
    List(String, LengthExpr),
    Switch,
    Fd,
}

#[derive(Debug)]
enum LengthExpr {
    Fixed(u32),
    None,
}

#[derive(Debug)]
struct Request {
    name: String,
    opcode: u32,
    fields: Vec<Field>,
    reply: Option<Reply>,
}

#[derive(Debug)]
struct Reply {
    fields: Vec<Field>,
}

#[derive(Debug)]
struct Event {
    name: String,
    number: u32,
    fields: Vec<Field>,
}

fn parse_fields(node: Node) -> Vec<Field> {
    let mut fields = Vec::new();

    let mut pad_index = 0;
    for child in node.children() {
        if child.is_element() {
            match child.tag_name().name() {
                "field" => {
                    let field_name = sanitize(child.attribute("name").unwrap()).to_string();
                    let field_type = child.attribute("type").unwrap().to_string();
                    fields.push(Field {
                        name: field_name,
                        type_: FieldType::Name(field_type),
                    });
                }
                "pad" => {
                    if let Some(bytes) = child.attribute("bytes") {
                        let padding = u32::from_str(bytes).unwrap();
                        fields.push(Field {
                            name: format!("pad{pad_index}"),
                            type_: FieldType::Padding(padding),
                        });
                        pad_index += 1;
                    }
                }
                "list" => {
                    let length = if let Some(expr) = child.first_element_child() {
                        match expr.tag_name().name() {
                            "value" => {
                                LengthExpr::Fixed(u32::from_str(expr.text().unwrap()).unwrap())
                            }
                            _ => {
                                // TODO: handle other types of list length expressions
                                continue;
                            }
                        }
                    } else {
                        LengthExpr::None
                    };

                    let field_name = sanitize(child.attribute("name").unwrap()).to_string();
                    let field_type = child.attribute("type").unwrap().to_string();

                    fields.push(Field {
                        name: field_name,
                        type_: FieldType::List(field_type, length),
                    });
                }
                "switch" => {
                    let field_name = sanitize(child.attribute("name").unwrap()).to_string();
                    fields.push(Field {
                        name: field_name,
                        type_: FieldType::Switch,
                    });
                }
                "fd" => {
                    let field_name = sanitize(child.attribute("name").unwrap()).to_string();
                    fields.push(Field {
                        name: field_name,
                        type_: FieldType::Fd,
                    });
                }
                _ => {}
            }
        }
    }

    fields
}

fn gen_fields(writer: &mut impl Write, header_name: &str, ast: &Ast, fields: &[Field]) {
    for field in fields {
        let field_name = &field.name;
        let field_type = match &field.type_ {
            FieldType::Name(type_name) => ast
                .lookup(header_name, type_name)
                .unwrap_or_else(|| panic!("{}", type_name))
                .to_string(),
            FieldType::Padding(padding) => {
                if *padding == 1 {
                    "u8".to_string()
                } else {
                    format!("[u8; {padding}]")
                }
            }
            FieldType::List(type_name, LengthExpr::Fixed(length)) => {
                let resolved_type = ast
                    .lookup(header_name, type_name)
                    .unwrap_or_else(|| panic!("{}", type_name))
                    .to_string();
                format!("[{resolved_type}; {length}]")
            }
            FieldType::List(_, LengthExpr::None) | FieldType::Switch | FieldType::Fd => {
                // Don't generate struct fields for variable-length lists, switches, or file descriptors
                continue;
            }
        };
        writeln!(writer, "        pub {field_name}: {field_type},").unwrap();
    }
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
        let extension_name = root.attribute("extension-name").map(|s| s.to_string());
        let major_version = root.attribute("major-version").map(|s| s.to_string());
        let minor_version = root.attribute("minor-version").map(|s| s.to_string());

        let mut prefix = "xcb".to_string();
        if let Some(ext_name) = &extension_name {
            prefix.push('_');
            prefix.push_str(&convert_extension_name(&ext_name));
        }

        let mut imports = Vec::new();
        let mut types = BTreeMap::new();
        let mut requests = Vec::new();
        let mut events = Vec::new();

        for child in root.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "import" => {
                        imports.push(child.text().unwrap().to_string());
                    }
                    "xidtype" | "xidunion" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let type_name = join(&[&prefix, &convert_name(&name), "t"]);
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
                        let type_name = join(&[&prefix, &convert_name(&name), "t"]);

                        let mut items = Vec::new();
                        for child in child.children() {
                            if child.is_element() {
                                if child.tag_name().name() == "item" {
                                    let item_name = child.attribute("name").unwrap();
                                    let full_item_name = join(&[
                                        &prefix,
                                        &convert_name(&name),
                                        &convert_name(item_name),
                                    ])
                                    .to_uppercase();

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
                        let type_name = join(&[&prefix, &convert_name(&name), "t"]);
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
                        let type_name = join(&[&prefix, &convert_name(&name), "t"]);
                        let fields = parse_fields(child);
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
                        let type_name = join(&[&prefix, &convert_name(&name), "t"]);
                        let fields = parse_fields(child);
                        types.insert(
                            name,
                            Type {
                                name: type_name,
                                kind: Kind::Union { fields },
                            },
                        );
                    }
                    "request" => {
                        let name = convert_name(child.attribute("name").unwrap());
                        let opcode = u32::from_str(child.attribute("opcode").unwrap()).unwrap();

                        let fields = parse_fields(child);

                        let mut reply = None;
                        for node in child.children() {
                            if node.is_element() && node.tag_name().name() == "reply" {
                                let reply_fields = parse_fields(node);
                                reply = Some(Reply {
                                    fields: reply_fields,
                                });
                            }
                        }

                        requests.push(Request {
                            name,
                            opcode,
                            fields,
                            reply,
                        });
                    }
                    "event" => {
                        let name = convert_name(child.attribute("name").unwrap());
                        let number = u32::from_str(child.attribute("number").unwrap()).unwrap();
                        let fields = parse_fields(child);
                        events.push(Event {
                            name,
                            number,
                            fields,
                        });
                    }
                    _ => {}
                }
            }
        }

        modules.insert(
            header_name,
            Module {
                extension_name,
                prefix,
                major_version,
                minor_version,
                imports,
                types,
                requests,
                events,
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

            let prefix = &module.prefix;
            writeln!(writer, "    extern \"C\" {{").unwrap();
            writeln!(writer, "        pub static {prefix}_id: xcb_extension_t;").unwrap();
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

                    gen_fields(&mut writer, header_name, &ast, fields);

                    writeln!(writer, "    }}").unwrap();
                }
                Kind::Union { fields } => {
                    writeln!(writer, "    #[repr(C)]").unwrap();
                    writeln!(writer, "    #[derive(Copy, Clone)]").unwrap();
                    writeln!(writer, "    pub union {type_name} {{").unwrap();

                    gen_fields(&mut writer, header_name, &ast, fields);

                    writeln!(writer, "    }}").unwrap();
                }
            }
        }

        for request in &module.requests {
            let request_name = join(&[&module.prefix, &request.name]);

            let opcode_name = request_name.to_uppercase();
            let opcode = request.opcode;
            writeln!(writer, "    pub const {opcode_name}: u32 = {opcode};").unwrap();

            // Request struct
            writeln!(writer, "    #[repr(C)]").unwrap();
            writeln!(writer, "    #[derive(Copy, Clone)]").unwrap();
            writeln!(writer, "    pub struct {request_name}_request_t {{").unwrap();

            if module.extension_name.is_some() {
                writeln!(writer, "        pub major_opcode: u8,").unwrap();
                writeln!(writer, "        pub minor_opcode: u8,").unwrap();
                writeln!(writer, "        pub length: u16,").unwrap();
                gen_fields(&mut writer, header_name, &ast, &request.fields);
            } else {
                writeln!(writer, "        pub major_opcode: u8,").unwrap();
                if let Some(first) = request.fields.get(..1) {
                    gen_fields(&mut writer, header_name, &ast, first);
                } else {
                    writeln!(writer, "        pub pad0: [u8; 1],").unwrap();
                }
                writeln!(writer, "        pub length: u16,").unwrap();
                if let Some(rest) = request.fields.get(1..) {
                    gen_fields(&mut writer, header_name, &ast, rest);
                }
            }

            writeln!(writer, "    }}").unwrap();

            if let Some(reply) = &request.reply {
                // Reply struct
                writeln!(writer, "    #[repr(C)]").unwrap();
                writeln!(writer, "    #[derive(Copy, Clone)]").unwrap();
                writeln!(writer, "    pub struct {request_name}_reply_t {{").unwrap();
                writeln!(writer, "        pub response_type: u8,").unwrap();
                if let Some(first) = reply.fields.get(..1) {
                    gen_fields(&mut writer, header_name, &ast, first);
                } else {
                    writeln!(writer, "        pub pad0: [u8; 1],").unwrap();
                }
                writeln!(writer, "        pub sequence: u16,").unwrap();
                writeln!(writer, "        pub length: u32,").unwrap();
                if let Some(rest) = reply.fields.get(1..) {
                    gen_fields(&mut writer, header_name, &ast, rest);
                }
                writeln!(writer, "    }}").unwrap();

                // Cookie struct
                writeln!(writer, "    #[repr(C)]").unwrap();
                writeln!(writer, "    #[derive(Copy, Clone)]").unwrap();
                writeln!(writer, "    pub struct {request_name}_cookie_t {{").unwrap();
                writeln!(writer, "        pub sequence: c_uint,").unwrap();
                writeln!(writer, "    }}").unwrap();
            }

            let mut args = Vec::<u8>::new();
            writeln!(args, "            c: *mut xcb_connection_t,").unwrap();

            for field in &request.fields {
                let field_name = &field.name;
                match &field.type_ {
                    FieldType::Name(type_name) => {
                        let field_type = ast
                            .lookup(header_name, type_name)
                            .unwrap_or_else(|| panic!("{}", type_name))
                            .to_string();
                        writeln!(args, "            {field_name}: {field_type},").unwrap();
                    }
                    FieldType::List(type_name, _) => {
                        let resolved_type = ast
                            .lookup(header_name, type_name)
                            .unwrap_or_else(|| panic!("{}", type_name))
                            .to_string();

                        writeln!(args, "            {field_name}_len: u32,").unwrap();
                        writeln!(args, "            {field_name}: *const {resolved_type},")
                            .unwrap();
                    }
                    FieldType::Switch => {
                        writeln!(args, "            {field_name}: *const c_void,").unwrap();
                    }
                    FieldType::Fd => {
                        writeln!(args, "            {field_name}: i32,").unwrap();
                    }
                    FieldType::Padding(_) => {
                        continue;
                    }
                };
            }

            let (cookie_type, checked, unchecked) = if request.reply.is_some() {
                (&*request_name, "", "_unchecked")
            } else {
                ("xcb_void", "_checked", "")
            };

            writeln!(writer, "    extern \"C\" {{").unwrap();

            writeln!(writer, "        pub fn {request_name}{checked}(").unwrap();
            writer.write(&args).unwrap();
            writeln!(writer, "        ) -> {cookie_type}_cookie_t;").unwrap();

            writeln!(writer, "        pub fn {request_name}{unchecked}(").unwrap();
            writer.write(&args).unwrap();
            writeln!(writer, "        ) -> {cookie_type}_cookie_t;").unwrap();

            if request.reply.is_some() {
                writeln!(writer, "        pub fn {request_name}_reply(").unwrap();
                writeln!(writer, "            c: *mut xcb_connection_t,").unwrap();
                writeln!(writer, "            cookie: {cookie_type}_cookie_t,").unwrap();
                writeln!(writer, "            e: *mut *mut xcb_generic_error_t,").unwrap();
                writeln!(writer, "        ) -> *mut {request_name}_reply_t;").unwrap();
            }

            writeln!(writer, "    }}").unwrap();
        }

        for event in &module.events {
            let event_name = &event.name;

            let number_name = event_name.to_uppercase();
            let number = event.number;
            writeln!(writer, "    pub const {number_name}: u32 = {number};").unwrap();

            writeln!(writer, "    #[repr(C)]").unwrap();
            writeln!(writer, "    #[derive(Copy, Clone)]").unwrap();
            writeln!(writer, "    pub struct {event_name}_event_t {{").unwrap();
            writeln!(writer, "        pub response_type: u8,").unwrap();
            if let Some(first) = event.fields.get(..1) {
                gen_fields(&mut writer, header_name, &ast, first);
            } else {
                writeln!(writer, "        pub pad0: [u8; 1],").unwrap();
            }
            writeln!(writer, "        pub sequence: u16,").unwrap();
            if let Some(rest) = event.fields.get(1..) {
                gen_fields(&mut writer, header_name, &ast, rest);
            }
            writeln!(writer, "    }}").unwrap();
        }

        writeln!(writer, "}}").unwrap();
    }
}
