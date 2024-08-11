pub mod vcf_util;
pub mod forge;
pub mod filter;

use std::path::PathBuf;
use env_logger::Env;
use log::info;
use structopt::StructOpt;

use crate::filter::filter;

#[derive(Debug, StructOpt)]
#[structopt(name = "forger", about = "VCF manipulation based on FORGe ranking.")]
struct Opt {
    /// Enable verbose mode
    // short and long flags (-v, --verbose) will be deduced
    #[structopt(short, long, global = true)]
    verbose: bool,

    /// FORGe rank file
    #[structopt(
        short,
        long,
        global = true,
        parse(from_os_str),
        default_value = "ordered.txt"
    )]
    forge_rank: PathBuf,

    /// Gzip output, detected by file extension by default
    #[structopt(short, long, global = true)]
    gzip: bool,

    /// Output file, stdout if not present
    #[structopt(short, long, global = true, parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(subcommand)]
    #[allow(dead_code)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "filter")]
    /// Filter VCF records based on FORGe ranking
    Filter {
        /// Input VCF file
        #[structopt(parse(from_os_str))]
        input: PathBuf,

        /// Top percentage of records to keep
        #[structopt(short, long, default_value = "0.1")]
        top: f64,
    },
}

fn init_logger(verbose: bool) {
    let level = if verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(Env::default().default_filter_or(level)).init();
}

fn main() {
    let opt = Opt::from_args();
    init_logger(opt.verbose);

    let output = opt.output.unwrap_or(PathBuf::from("-"));
    info!("parameter: verbose\t\t= {}", opt.verbose);
    info!("parameter: forge_rank\t= {}", &opt.forge_rank.display());
    info!("parameter: output\t\t= {}", &output.display());

    match opt.cmd {
        Command::Filter { top, input } => {
            info!("parameter: top\t\t= {}", top);
            filter(input, opt.forge_rank, top, output, opt.gzip);
        }
    }
}
