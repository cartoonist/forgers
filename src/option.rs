use std::path::PathBuf;
use structopt::StructOpt;

/// Data structure for command line options.
#[derive(Debug, StructOpt)]
#[structopt(name = "forgers", about = "VCF manipulation based on FORGe ranking.")]
pub struct Opt {
    /// Enable verbose mode
    #[structopt(short, long, global = true)]
    pub verbose: bool,

    /// Input VCF file, stdin if not specified
    #[structopt(global = true, default_value = "-", parse(from_os_str))]
    pub input: PathBuf,

    /// FORGe rank file
    #[structopt(
        short,
        long,
        global = true,
        parse(from_os_str),
        default_value = "ordered.txt"
    )]
    pub ranks_path: PathBuf,

    /// Gzip output, detected by file extension by default
    #[structopt(short, long, global = true)]
    pub gzip: bool,

    /// Output file, stdout if not specified
    #[structopt(short, long, global = true, default_value = "-", parse(from_os_str))]
    pub output: PathBuf,

    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "filter")]
    /// Filter VCF records based on FORGe ranking
    Filter {
        /// Top fraction of records to keep, keeps all by default
        #[structopt(short, long, default_value = "1.0")]
        top: f64,

        /// Annotate the filtered records with FORGe rank
        #[structopt(short, long)]
        annotate: bool,

        /// Annotate key for INFO field
        #[structopt(short = "k", long, default_value = "FORGE")]
        info_key: String,
    },
    /// Resolve overlapping variants based on FORGe ranking
    Resolve {},
}
