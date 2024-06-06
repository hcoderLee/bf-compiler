use crate::program::Program;

mod program;

fn main() -> std::io::Result<()> {
    let file_name = std::env::args().nth(1).unwrap();
    let source_code = std::fs::read(file_name)?;

    Program::new(&source_code).run()
}
