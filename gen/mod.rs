use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::str::FromStr;

use roxmltree::{Document, Node};

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
    fn resolve_type_name(&self, module: &Module, type_name: &str) -> String {
        let (module, name) = if let Some(colon) = type_name.find(':') {
            let module_name = &type_name[0..colon];
            let name = &type_name[colon + 1..];
            let module = &self.modules[module_name];
            (Some(module), name)
        } else {
            (self.find_module_for_type(module, type_name), type_name)
        };

        if let Some(module) = module {
            let prefix = if let Some(ext_name) = &module.extension_name {
                convert_extension_name(ext_name) + "_"
            } else {
                String::new()
            };

            return format!("xcb_{prefix}{}_t", convert_name(name));
        }

        if let Some(name) = self.global_types.get(type_name) {
            return name.to_string();
        }

        panic!("couldn't resolve type name {type_name}");
    }

    fn find_module_for_type<'a>(
        &'a self,
        module: &'a Module,
        type_name: &str,
    ) -> Option<&'a Module> {
        if module.types.contains_key(type_name) {
            return Some(module);
        }

        for import in &module.imports {
            if let Some(result) = self.find_module_for_type(&self.modules[import], type_name) {
                return Some(result);
            }
        }

        None
    }

    fn find_module_for_event<'a>(
        &'a self,
        module: &'a Module,
        event_name: &str,
    ) -> Option<&'a Module> {
        if module.events.iter().any(|event| event.name == event_name) {
            return Some(module);
        }

        for import in &module.imports {
            if let Some(result) = self.find_module_for_event(&self.modules[import], event_name) {
                return Some(result);
            }
        }

        None
    }

    fn find_module_for_error<'a>(
        &'a self,
        module: &'a Module,
        error_name: &str,
    ) -> Option<&'a Module> {
        if module.errors.iter().any(|error| error.name == error_name) {
            return Some(module);
        }

        for import in &module.imports {
            if let Some(result) = self.find_module_for_error(&self.modules[import], error_name) {
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
    requests: Vec<Request>,
    events: Vec<Event>,
    errors: Vec<Error>,
}

#[derive(Debug)]
enum Type {
    Id,
    Enum { items: Vec<(String, u32)> },
    TypeDef { value: String },
    Struct { fields: Vec<Field> },
    Union { fields: Vec<Field> },
    EventStruct(EventStruct),
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
    List(String, Length),
    Switch,
    Fd,
}

#[derive(Debug)]
enum Length {
    Fixed(u32),
    FieldRef,
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
    inner: EventInner,
}

#[derive(Debug)]
enum EventInner {
    Event {
        xge: bool,
        sequence: bool,
        fields: Vec<Field>,
    },
    Copy {
        ref_: String,
    },
}

#[derive(Debug)]
struct EventStruct {
    extension: String,
    xge: bool,
    opcode_min: u32,
    opcode_max: u32,
}

#[derive(Debug)]
struct Error {
    name: String,
    number: u32,
    inner: ErrorInner,
}

#[derive(Debug)]
enum ErrorInner {
    Error { fields: Vec<Field> },
    Copy { ref_: String },
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
                        if expr.tag_name().name() == "value" {
                            Length::Fixed(u32::from_str(expr.text().unwrap()).unwrap())
                        } else {
                            Length::FieldRef
                        }
                    } else {
                        Length::None
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

fn gen_fields(w: &mut impl Write, module: &Module, ast: &Ast, fields: &[Field]) {
    for field in fields {
        let field_name = &field.name;
        let field_type = match &field.type_ {
            FieldType::Name(type_name) => ast.resolve_type_name(module, type_name),
            FieldType::Padding(padding) => {
                if *padding == 1 {
                    "u8".to_string()
                } else {
                    format!("[u8; {padding}]")
                }
            }
            FieldType::List(type_name, Length::Fixed(length)) => {
                let resolved_type = ast.resolve_type_name(module, type_name);
                format!("[{resolved_type}; {length}]")
            }
            FieldType::List(..) | FieldType::Switch | FieldType::Fd => {
                // Don't generate struct fields for variable-length lists, switches, or file descriptors
                continue;
            }
        };
        writeln!(w, "        pub {field_name}: {field_type},").unwrap();
    }
}

fn gen_iterator(w: &mut impl Write, prefix: &str, name: &str) {
    writeln!(w, "    #[repr(C)]").unwrap();
    writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
    writeln!(w, "    pub struct xcb_{prefix}{name}_iterator_t {{").unwrap();
    writeln!(w, "        pub data: *mut xcb_{prefix}{name}_t,").unwrap();
    writeln!(w, "        pub rem: c_int,").unwrap();
    writeln!(w, "        pub index: c_int,").unwrap();
    writeln!(w, "    }}").unwrap();

    writeln!(w, "    extern \"C\" {{").unwrap();
    writeln!(w, "        pub fn xcb_{prefix}{name}_next(").unwrap();
    writeln!(w, "            i: *mut xcb_{prefix}{name}_iterator_t,").unwrap();
    writeln!(w, "        );").unwrap();
    writeln!(w, "        pub fn xcb_{prefix}{name}_end(").unwrap();
    writeln!(w, "            i: xcb_{prefix}{name}_iterator_t,").unwrap();
    writeln!(w, "        ) -> xcb_generic_iterator_t;").unwrap();
    writeln!(w, "    }}").unwrap();
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
        ("fd", "i32"),
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

        let mut imports = Vec::new();
        let mut types = BTreeMap::new();
        let mut requests = Vec::new();
        let mut events = Vec::new();
        let mut errors = Vec::new();

        for child in root.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "import" => {
                        imports.push(child.text().unwrap().to_string());
                    }
                    "xidtype" | "xidunion" => {
                        let name = child.attribute("name").unwrap().to_string();
                        types.insert(name, Type::Id);
                    }
                    "enum" => {
                        let name = child.attribute("name").unwrap().to_string();

                        let mut items = Vec::new();
                        for child in child.children() {
                            if child.is_element() {
                                if child.tag_name().name() == "item" {
                                    let item_name = child.attribute("name").unwrap().to_string();
                                    let choice = child.first_element_child().unwrap();
                                    let value = match choice.tag_name().name() {
                                        "value" => u32::from_str(choice.text().unwrap()).unwrap(),
                                        "bit" => {
                                            1 << u32::from_str(choice.text().unwrap()).unwrap()
                                        }
                                        _ => panic!(),
                                    };

                                    items.push((item_name, value));
                                }
                            }
                        }

                        types.insert(name, Type::Enum { items });
                    }
                    "typedef" => {
                        let name = child.attribute("newname").unwrap().to_string();
                        let value = child.attribute("oldname").unwrap().to_string();
                        types.insert(name, Type::TypeDef { value });
                    }
                    "struct" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let fields = parse_fields(child);
                        types.insert(name, Type::Struct { fields });
                    }
                    "union" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let fields = parse_fields(child);
                        types.insert(name, Type::Union { fields });
                    }
                    "request" => {
                        let name = child.attribute("name").unwrap().to_string();
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
                        let name = child.attribute("name").unwrap().to_string();
                        let number = u32::from_str(child.attribute("number").unwrap()).unwrap();
                        let xge = child
                            .attribute("xge")
                            .map_or(false, |x| bool::from_str(x).unwrap());
                        let sequence = !child
                            .attribute("no-sequence-number")
                            .map_or(false, |x| bool::from_str(x).unwrap());
                        let fields = parse_fields(child);
                        events.push(Event {
                            name,
                            number,
                            inner: EventInner::Event {
                                xge,
                                sequence,
                                fields,
                            },
                        });
                    }
                    "eventcopy" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let number = u32::from_str(child.attribute("number").unwrap()).unwrap();
                        let ref_ = child.attribute("ref").unwrap().to_string();
                        events.push(Event {
                            name,
                            number,
                            inner: EventInner::Copy { ref_ },
                        });
                    }
                    "eventstruct" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let allowed = child.first_element_child().unwrap();
                        let extension = allowed.attribute("extension").unwrap().to_string();
                        let xge = child
                            .attribute("xge")
                            .map_or(false, |x| bool::from_str(x).unwrap());
                        let opcode_min =
                            u32::from_str(allowed.attribute("opcode-min").unwrap()).unwrap();
                        let opcode_max =
                            u32::from_str(allowed.attribute("opcode-max").unwrap()).unwrap();
                        types.insert(
                            name,
                            Type::EventStruct(EventStruct {
                                extension,
                                xge,
                                opcode_min,
                                opcode_max,
                            }),
                        );
                    }
                    "error" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let number_str = child.attribute("number").unwrap();
                        // XCB_GLX_GENERIC is -1
                        let number = if number_str == "-1" {
                            u8::MAX as u32
                        } else {
                            u32::from_str(number_str).unwrap()
                        };
                        let fields = parse_fields(child);
                        errors.push(Error {
                            name,
                            number,
                            inner: ErrorInner::Error { fields },
                        });
                    }
                    "errorcopy" => {
                        let name = child.attribute("name").unwrap().to_string();
                        let number = u32::from_str(child.attribute("number").unwrap()).unwrap();
                        let ref_ = child.attribute("ref").unwrap().to_string();
                        errors.push(Error {
                            name,
                            number,
                            inner: ErrorInner::Copy { ref_ },
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
                major_version,
                minor_version,
                imports,
                types,
                requests,
                events,
                errors,
            },
        );
    }

    let ast = Ast {
        global_types,
        modules,
    };

    let mut w = BufWriter::new(File::create(out_path).unwrap());
    for (header_name, module) in &ast.modules {
        let prefix = if let Some(ext_name) = &module.extension_name {
            convert_extension_name(ext_name) + "_"
        } else {
            String::new()
        };

        writeln!(w, "pub mod {header_name} {{").unwrap();

        writeln!(w, "    use super::*;").unwrap();

        for import in &module.imports {
            writeln!(w, "    use super::{import}::*;").unwrap();
        }

        if let Some(extension_name) = &module.extension_name {
            let extension_name_uppercase = extension_name.to_uppercase();

            if let Some(major_version) = &module.major_version {
                writeln!(
                    w,
                    "    pub const XCB_{extension_name_uppercase}_MAJOR_VERSION: u32 = {major_version};"
                )
                .unwrap();
            }

            if let Some(minor_version) = &module.minor_version {
                writeln!(
                    w,
                    "    pub const XCB_{extension_name_uppercase}_MINOR_VERSION: u32 = {minor_version};"
                )
                .unwrap();
            }

            writeln!(w, "    extern \"C\" {{").unwrap();
            writeln!(w, "        pub static xcb_{prefix}id: xcb_extension_t;").unwrap();
            writeln!(w, "    }}").unwrap();
        }

        let mut id_names = BTreeSet::new();
        for (name, type_) in &module.types {
            if let Type::Id = &type_ {
                id_names.insert(convert_name(name));
            }
        }

        for (type_name, type_) in &module.types {
            let name = convert_name(type_name);
            match &type_ {
                Type::Id => {
                    writeln!(w, "    pub type xcb_{prefix}{name}_t = u32;").unwrap();
                    gen_iterator(&mut w, &prefix, &name);
                }
                Type::Enum { items } => {
                    // Some source files contain duplicate xidtype and enum declarations, so don't output an enum type
                    // alias if there's already one from the xidtype.
                    if !id_names.contains(&name) {
                        writeln!(w, "    pub type xcb_{prefix}{name}_t = u32;").unwrap();
                    }
                    for (item_name, value) in items {
                        let const_name = format!("xcb_{prefix}{name}_{}", convert_name(item_name))
                            .to_uppercase();
                        writeln!(
                            w,
                            "    pub const {const_name}: xcb_{prefix}{name}_t = {value};"
                        )
                        .unwrap();
                    }
                }
                Type::TypeDef { value } => {
                    let field_type = ast.resolve_type_name(module, value);
                    writeln!(w, "    pub type xcb_{prefix}{name}_t = {field_type};").unwrap();
                    gen_iterator(&mut w, &prefix, &name);
                }
                Type::Struct { fields } => {
                    writeln!(w, "    #[repr(C)]").unwrap();
                    writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
                    writeln!(w, "    pub struct xcb_{prefix}{name}_t {{").unwrap();
                    gen_fields(&mut w, module, &ast, fields);
                    writeln!(w, "    }}").unwrap();
                    gen_iterator(&mut w, &prefix, &name);
                }
                Type::Union { fields } => {
                    writeln!(w, "    #[repr(C)]").unwrap();
                    writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
                    writeln!(w, "    pub union xcb_{prefix}{name}_t {{").unwrap();
                    gen_fields(&mut w, module, &ast, fields);
                    writeln!(w, "    }}").unwrap();
                    gen_iterator(&mut w, &prefix, &name);
                }
                Type::EventStruct(EventStruct {
                    extension,
                    xge,
                    opcode_min,
                    opcode_max,
                }) => {
                    let mut events = Vec::new();
                    for module in ast.modules.values() {
                        if module.extension_name.as_ref() == Some(&extension) {
                            for event in &module.events {
                                let event_xge = match &event.inner {
                                    EventInner::Event { xge, .. } => *xge,
                                    EventInner::Copy { ref_ } => {
                                        let mut ref_xge = false;
                                        let ref_module =
                                            ast.find_module_for_event(module, ref_).unwrap();
                                        for event in &ref_module.events {
                                            if &event.name == ref_ {
                                                if let EventInner::Event { xge, .. } = &event.inner
                                                {
                                                    ref_xge = *xge;
                                                }
                                                break;
                                            }
                                        }
                                        ref_xge
                                    }
                                };

                                if event_xge == *xge
                                    && event.number >= *opcode_min
                                    && event.number < *opcode_max
                                {
                                    events.push(event);
                                }
                            }

                            break;
                        }
                    }
                    events.sort_by_key(|e| e.number);

                    writeln!(w, "    #[repr(C)]").unwrap();
                    writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
                    writeln!(w, "    pub union xcb_{prefix}{name}_t {{").unwrap();
                    for event in &events {
                        let event_name = convert_name(&event.name);
                        writeln!(
                            w,
                            "        pub {event_name}: xcb_{prefix}{event_name}_event_t,"
                        )
                        .unwrap();
                    }
                    writeln!(w, "        pub event_header: xcb_raw_generic_event_t,").unwrap();
                    writeln!(w, "    }}").unwrap();
                    gen_iterator(&mut w, &prefix, &name);
                }
            }
        }

        for request in &module.requests {
            let request_name = format!("xcb_{prefix}{}", convert_name(&request.name));

            let opcode_name = request_name.to_uppercase();
            let opcode = request.opcode;
            writeln!(w, "    pub const {opcode_name}: u32 = {opcode};").unwrap();

            // Request struct
            writeln!(w, "    #[repr(C)]").unwrap();
            writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
            writeln!(w, "    pub struct {request_name}_request_t {{").unwrap();

            if module.extension_name.is_some() {
                writeln!(w, "        pub major_opcode: u8,").unwrap();
                writeln!(w, "        pub minor_opcode: u8,").unwrap();
                writeln!(w, "        pub length: u16,").unwrap();
                gen_fields(&mut w, module, &ast, &request.fields);
            } else {
                writeln!(w, "        pub major_opcode: u8,").unwrap();
                if let Some(first) = request.fields.get(..1) {
                    gen_fields(&mut w, module, &ast, first);
                } else {
                    writeln!(w, "        pub pad0: [u8; 1],").unwrap();
                }
                writeln!(w, "        pub length: u16,").unwrap();
                if let Some(rest) = request.fields.get(1..) {
                    gen_fields(&mut w, module, &ast, rest);
                }
            }

            writeln!(w, "    }}").unwrap();

            if let Some(reply) = &request.reply {
                // Reply struct
                writeln!(w, "    #[repr(C)]").unwrap();
                writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
                writeln!(w, "    pub struct {request_name}_reply_t {{").unwrap();
                writeln!(w, "        pub response_type: u8,").unwrap();
                if let Some(first) = reply.fields.get(..1) {
                    gen_fields(&mut w, module, &ast, first);
                } else {
                    writeln!(w, "        pub pad0: [u8; 1],").unwrap();
                }
                writeln!(w, "        pub sequence: u16,").unwrap();
                writeln!(w, "        pub length: u32,").unwrap();
                if let Some(rest) = reply.fields.get(1..) {
                    gen_fields(&mut w, module, &ast, rest);
                }
                writeln!(w, "    }}").unwrap();

                // Cookie struct
                writeln!(w, "    #[repr(C)]").unwrap();
                writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
                writeln!(w, "    pub struct {request_name}_cookie_t {{").unwrap();
                writeln!(w, "        pub sequence: c_uint,").unwrap();
                writeln!(w, "    }}").unwrap();
            }

            let mut args = Vec::<u8>::new();
            writeln!(args, "            c: *mut xcb_connection_t,").unwrap();

            for field in &request.fields {
                let field_name = &field.name;
                match &field.type_ {
                    FieldType::Name(type_name) => {
                        let field_type = ast.resolve_type_name(module, type_name);
                        writeln!(args, "            {field_name}: {field_type},").unwrap();
                    }
                    FieldType::List(type_name, length) => {
                        let resolved_type = ast.resolve_type_name(module, type_name);
                        if let Length::None = length {
                            writeln!(args, "            {field_name}_len: u32,").unwrap();
                        }
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

            writeln!(w, "    extern \"C\" {{").unwrap();

            writeln!(w, "        pub fn {request_name}{checked}(").unwrap();
            w.write(&args).unwrap();
            writeln!(w, "        ) -> {cookie_type}_cookie_t;").unwrap();

            writeln!(w, "        pub fn {request_name}{unchecked}(").unwrap();
            w.write(&args).unwrap();
            writeln!(w, "        ) -> {cookie_type}_cookie_t;").unwrap();

            if request.reply.is_some() {
                writeln!(w, "        pub fn {request_name}_reply(").unwrap();
                writeln!(w, "            c: *mut xcb_connection_t,").unwrap();
                writeln!(w, "            cookie: {cookie_type}_cookie_t,").unwrap();
                writeln!(w, "            e: *mut *mut xcb_generic_error_t,").unwrap();
                writeln!(w, "        ) -> *mut {request_name}_reply_t;").unwrap();
            }

            writeln!(w, "    }}").unwrap();
        }

        for event in &module.events {
            let event_name = format!("xcb_{prefix}{}", convert_name(&event.name));

            let number_name = event_name.to_uppercase();
            let number = event.number;
            writeln!(w, "    pub const {number_name}: u32 = {number};").unwrap();

            match &event.inner {
                EventInner::Event {
                    fields,
                    xge,
                    sequence,
                } => {
                    writeln!(w, "    #[repr(C)]").unwrap();
                    writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
                    writeln!(w, "    pub struct {event_name}_event_t {{").unwrap();
                    if *xge {
                        writeln!(w, "        pub response_type: u8,").unwrap();
                        writeln!(w, "        pub extension: u8,").unwrap();
                        writeln!(w, "        pub sequence: u16,").unwrap();
                        writeln!(w, "        pub length: u32,").unwrap();
                        writeln!(w, "        pub event_type: u16,").unwrap();
                        gen_fields(&mut w, module, &ast, fields);
                    } else {
                        writeln!(w, "        pub response_type: u8,").unwrap();
                        if *sequence {
                            if let Some(first) = fields.get(..1) {
                                gen_fields(&mut w, module, &ast, first);
                            } else {
                                writeln!(w, "        pub pad0: [u8; 1],").unwrap();
                            }
                            writeln!(w, "        pub sequence: u16,").unwrap();
                            if let Some(rest) = fields.get(1..) {
                                gen_fields(&mut w, module, &ast, rest);
                            }
                        } else {
                            gen_fields(&mut w, module, &ast, fields);
                        }
                    }
                    writeln!(w, "    }}").unwrap();
                }
                EventInner::Copy { ref_ } => {
                    let ref_module = ast.find_module_for_event(module, ref_).unwrap();
                    let ref_prefix = if let Some(ext_name) = &ref_module.extension_name {
                        convert_extension_name(ext_name) + "_"
                    } else {
                        String::new()
                    };
                    let ref_name = format!("xcb_{ref_prefix}{}_event_t", convert_name(ref_));
                    writeln!(w, "    pub type {event_name}_event_t = {ref_name};").unwrap();
                }
            }
        }

        for error in &module.errors {
            let error_name = format!("xcb_{prefix}{}", convert_name(&error.name));

            let number_name = error_name.to_uppercase();
            let number = error.number;
            writeln!(w, "    pub const {number_name}: u32 = {number};").unwrap();

            match &error.inner {
                ErrorInner::Error { fields } => {
                    writeln!(w, "    #[repr(C)]").unwrap();
                    writeln!(w, "    #[derive(Copy, Clone)]").unwrap();
                    writeln!(w, "    pub struct {error_name}_error_t {{").unwrap();
                    writeln!(w, "        pub response_type: u8,").unwrap();
                    writeln!(w, "        pub error_code: u8,").unwrap();
                    writeln!(w, "        pub sequence: u16,").unwrap();
                    if fields.len() < 1 {
                        writeln!(w, "        pub bad_value: u32,").unwrap();
                    }
                    if fields.len() < 2 {
                        writeln!(w, "        pub minor_opcode: u16,").unwrap();
                    }
                    if fields.len() < 3 {
                        writeln!(w, "        pub major_opcode: u8,").unwrap();
                    }
                    gen_fields(&mut w, module, &ast, fields);
                    writeln!(w, "    }}").unwrap();
                }
                ErrorInner::Copy { ref_ } => {
                    let ref_module = ast.find_module_for_error(module, ref_).unwrap();
                    let ref_prefix = if let Some(ext_name) = &ref_module.extension_name {
                        convert_extension_name(ext_name) + "_"
                    } else {
                        String::new()
                    };
                    let ref_name = format!("xcb_{ref_prefix}{}_error_t", convert_name(ref_));
                    writeln!(w, "    pub type {error_name}_error_t = {ref_name};").unwrap();
                }
            }
        }

        writeln!(w, "}}").unwrap();
    }
}
