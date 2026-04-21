use anyhow::Result;

pub trait ResultExt<T> {
    fn print_and_exit(self) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn print_and_exit(self) -> Result<T> {
        if let Err(error) = &self {
            eprintln!("error: {error:#}");
        }

        self
    }
}
