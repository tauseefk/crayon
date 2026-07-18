use crate::resource::Resource;
use clap::Parser;

/// Which `assets/documents/<name>.json` to open, from the `--doc <name>` dev flag.
#[derive(Parser)]
#[command(name = "crayon")]
pub struct LaunchOptions {
    /// Document name under `assets/documents/` to open on launch.
    #[allow(dead_code)]
    #[arg(long = "doc", default_value = "default")]
    pub document: String,
}

impl LaunchOptions {
    pub fn from_args() -> Self {
        Self::parse()
    }
}

impl Resource for LaunchOptions {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_doc_flag() {
        let opts = LaunchOptions::try_parse_from(["crayon", "--doc", "two-boards"]).unwrap();
        assert_eq!(opts.document, "two-boards");
    }

    #[test]
    fn defaults_without_flag() {
        let opts = LaunchOptions::try_parse_from(["crayon"]).unwrap();
        assert_eq!(opts.document, "default");
    }

    #[test]
    fn errors_on_doc_without_value() {
        assert!(LaunchOptions::try_parse_from(["crayon", "--doc"]).is_err());
    }
}
