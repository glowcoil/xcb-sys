use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use roxmltree::Document;

pub fn gen(sources: &[&str], out_path: &Path) {
    let mut writer = BufWriter::new(File::create(out_path).unwrap());

    for source in sources {
        let mut path = Path::new("xml").join(source);
        path.set_extension("xml");
        let bytes = fs::read(path).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let _tree = Document::parse(text).unwrap();

        write!(writer, "pub mod {source} {{}}").unwrap();
    }
}
