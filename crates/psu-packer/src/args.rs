use chrono::NaiveDateTime;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(version, about = "", arg_required_else_help(true))]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum Commands {
    /// Create a .psu from scratch
    Create(CreateArgs),
    /// Read the content of a psu
    Read(ReadArgs),
    /// Provide a .toml file to automate the creation of multiple .psu files.
    ///
    /// For example you toml can contain one or many of the following block:
    ///
    ///   [[psu]]
    ///   name="APP_FOOBAR"                   # the name of the folder the psu will unpack to
    ///   files=["./icon.sys", "./list.icn"]  # you can use paths relative to the path of this toml
    ///   output="APP_FOOBARv2.psu"           # optional, if omitted, the output file will be {name}.psu
    ///   timestamp=2024-10-10T10:30:00       # optional, if omitted, current time will be used
    #[clap(verbatim_doc_comment)]
    Automate(AutomateArgs),
    /// Add one or many files to a .psu
    Add(AddArgs),
    /// Remove one or many entries from a .psu
    Delete(DeleteArgs),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct CreateArgs {
    #[arg(value_name = "FILES", help = "One or many files to add to the psu")]
    pub(crate) files: Vec<String>,

    #[arg(
        short = 'n',
        long,
        value_name = "STRING",
        help = "Name of the psu folder"
    )]
    pub(crate) name: String,

    #[arg(
        short = 'o',
        long,
        value_name = "PATH",
        help = "Output path, uses {name}.psu by default"
    )]
    pub(crate) output: Option<String>,

    #[arg(
        short = 't',
        long,
        value_name = "PATH",
        help = "The timestamp to be applied to files in the psu"
    )]
    pub(crate) timestamp: Option<NaiveDateTime>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct ReadArgs {
    #[arg(value_name = "FILE", help = "Path of the psu to read")]
    pub(crate) file: String,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct AutomateArgs {
    #[arg(value_name = "FILE", help = "Path of the .toml to use")]
    pub(crate) toml: String,

    #[arg(
        short = 'o',
        long,
        help = "If this flag is provided, any existing .psu will be overwritten"
    )]
    pub(crate) overwrite: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct AddArgs {
    #[arg(long, value_name = "FILE", help = "Path of the psu to add entries to")]
    pub(crate) psu: String,

    #[arg(
        value_name = "FILES",
        help = "One or many files to be added to the psu"
    )]
    pub(crate) files: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct DeleteArgs {
    #[arg(
        long,
        value_name = "FILE",
        help = "Path of the psu to remove entries from"
    )]
    pub(crate) psu: String,

    #[arg(
        value_name = "ENTRIES",
        help = "One or many entries to be removed from the psu"
    )]
    pub(crate) entries: Vec<String>,
}
