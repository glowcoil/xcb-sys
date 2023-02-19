use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use roxmltree::Document;

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
                let mut tmp = String::new();
                tmp.extend(c.to_lowercase());

                while let Some(c) = chars.next_if(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                {
                    tmp.extend(c.to_lowercase());
                }

                if let Some(next) = chars.peek() {
                    if next.is_ascii_lowercase() {
                        continue;
                    }
                }

                if !first {
                    out.push('_');
                }
                first = false;

                out.push_str(&tmp);
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

pub fn gen(sources: &[&str], out_path: &Path) {
    let mut writer = BufWriter::new(File::create(out_path).unwrap());

    for source in sources {
        let mut path = Path::new("xml").join(source);
        path.set_extension("xml");
        let bytes = fs::read(path).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let tree = Document::parse(text).unwrap();
        let root = tree.root_element();

        if !root.has_tag_name("xcb") {
            panic!();
        }

        writeln!(writer, "pub mod {source} {{").unwrap();

        for child in root.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "struct" => {
                        let name = convert_name(child.attribute("name").unwrap());
                        writeln!(writer, "pub struct xcb_{name}_t {{}}").unwrap();
                    }
                    _ => {}
                }
            }
        }

        writeln!(writer, "}}").unwrap();
    }
}
