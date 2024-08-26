pub mod filter;
pub mod forge;
pub mod option;
pub mod resolve;
pub mod vcf_util;

use env_logger::Env;
use log::info;
use std::io::{BufReader, BufWriter, Read, Write};
use structopt::StructOpt;
use vcf::{VCFReader, VCFWriter};

use crate::vcf_util::path_or;

/// Initial the logger and set the verbosity.
fn init_logger(verbose: bool) {
    let level = if verbose { "debug" } else { "warn" };
    env_logger::Builder::from_env(Env::default().default_filter_or(level)).init();
}

impl<W: Write, R: Read> vcf_util::Process<W, R> for option::Opt {
    /// Dispatch the function corresponding to each subcommand with required parameters.
    fn process(&mut self, vcf_writer: VCFWriter<BufWriter<W>>, vcf_reader: VCFReader<BufReader<R>>)
    where
        R: Read,
        W: Write,
    {
        match &self.cmd {
            option::Command::Filter {
                top,
                annotate,
                info_key,
            } => {
                info!("parameter: top\t\t= {}", top);
                info!("parameter: annotate\t= {}", annotate);
                info!("parameter: info_key\t= {}", info_key);
                info!("parameter: command\t\t= filter");
                filter::filter(
                    vcf_writer,
                    vcf_reader,
                    &self.ranks_path,
                    *top,
                    *annotate,
                    info_key,
                )
                .unwrap();
            }

            option::Command::Resolve {} => {
                info!("parameter: command\t\t= resolve");
                resolve::resolve(vcf_writer, vcf_reader, &self.ranks_path).unwrap();
            }
        }
    }
}

fn main() {
    let opt = option::Opt::from_args();
    init_logger(opt.verbose);

    info!("parameter: verbose\t\t= {}", opt.verbose);
    info!("parameter: input\t\t= {}", path_or(&opt.input, "stdin"));
    info!("parameter: ranks_path\t= {}", &opt.ranks_path.display());
    info!("parameter: gzip\t\t= {}", opt.gzip);
    info!("parameter: output\t\t= {}", path_or(&opt.output, "stdout"));

    vcf_util::launch_iostream(opt);
}
