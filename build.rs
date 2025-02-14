use std::path::PathBuf;
use syntect::dumps::dump_to_uncompressed_file;
use syntect::parsing::SyntaxSetBuilder;

fn main() {
    let current_dir = env!("CARGO_MANIFEST_DIR");
    let assets_dir = PathBuf::from(current_dir).join("assets").join("syntaxes");
    let output_file = assets_dir.join("syntaxes.packdump");

    if !output_file.exists() {
        let mut builder = SyntaxSetBuilder::new();
        builder.add_from_folder(&assets_dir, true).unwrap();
        let ss = builder.build();
        dump_to_uncompressed_file(&ss, output_file).unwrap();
    }
}
