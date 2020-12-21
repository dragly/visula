use std::error::Error;
use std::process::Command;

trait Verify {
    fn verify(&mut self) -> Result<(), Box<dyn Error>>;
}

impl Verify for Command {
    fn verify(&mut self) -> Result<(), Box<dyn Error>> {
        let output = self.output()?;
        match output.status.success() {
            true => Ok(()),
            false => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8(output.stderr)?.as_str(),
            ))),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/shader.vert");
    println!("cargo:rerun-if-changed=src/shader.frag");
    println!("cargo:rerun-if-changed=src/mesh.vert");
    println!("cargo:rerun-if-changed=src/mesh.frag");

    Command::new("glslc")
        .args(&["src/shader.vert", "-o", "src/shader.vert.spv"])
        .verify()?;
    Command::new("glslc")
        .args(&["src/shader.frag", "-o", "src/shader.frag.spv"])
        .verify()?;
    Command::new("glslc")
        .args(&["src/mesh.vert", "-o", "src/mesh.vert.spv"])
        .verify()?;
    Command::new("glslc")
        .args(&["src/mesh.frag", "-o", "src/mesh.frag.spv"])
        .verify()?;

    Ok(())
}
