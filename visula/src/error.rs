#[derive(Clone, Debug)]
pub struct Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Visula error")
    }
}

impl std::error::Error for Error {}
