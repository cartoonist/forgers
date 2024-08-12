pub mod filter;
pub mod forge;
pub mod vcf_util;

use env_logger::Env;
use log::info;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::filter::filter;

#[derive(Debug, StructOpt)]
#[structopt(name = "forgers", about = "VCF manipulation based on FORGe ranking.")]
struct Opt {
    /// Enable verbose mode
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
    #[structopt(short, long, global = true, default_value = "-", parse(from_os_str))]
    output: PathBuf,

    #[structopt(subcommand)]
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

        /// Top fraction of records to keep, keeps all by default
        #[structopt(short, long, default_value = "1.0")]
        top: f64,
    },
}

fn init_logger(verbose: bool) {
    let level = if verbose { "debug" } else { "warn" };
    env_logger::Builder::from_env(Env::default().default_filter_or(level)).init();
}

fn main() {
    let opt = Opt::from_args();
    init_logger(opt.verbose);

    info!("parameter: verbose\t\t= {}", opt.verbose);
    info!("parameter: forge_rank\t= {}", &opt.forge_rank.display());
    info!("parameter: output\t\t= {}", &opt.output.display());

    match opt.cmd {
        Command::Filter { input, top } => {
            info!("parameter: top\t\t= {}", top);
            filter(opt.output, input, opt.forge_rank, top, opt.gzip);
        }
    }
}
