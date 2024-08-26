use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use vcf::{VCFError, VCFReader, VCFRecord, VCFWriter};

use crate::forge;

/// Filter and annotate VCF records based on FORGe ranking.
///
/// # Arguments
///
/// * `vcf_reader` - VCF input stream
/// * `vcf_writer` - VCF output stream
/// * `ranks_path` - FORGe ranking file path
/// * `top` - This fraction of records will be written in the output stream
/// * `annotate` - Whether annotate the records with FORGe ranking or not
/// * `info_key` - VCF INFO key for FORGe ranking annotation
pub fn filter<T, W, R>(
    mut vcf_writer: VCFWriter<BufWriter<W>>,
    mut vcf_reader: VCFReader<BufReader<R>>,
    forge_rank: &T,
    top: f64,
    annotate: bool,
    info_key: &String,
) -> Result<(), VCFError>
where
    T: AsRef<Path>,
    W: Write,
    R: Read,
{
    let ranks = forge::load_rank(forge_rank, top);
    let mut vcf_record = VCFRecord::new(vcf_reader.header().clone());
    loop {
        let fetched = vcf_reader.next_record(&mut vcf_record)?;
        if fetched {
            match forge::forge_rank(&vcf_record, &ranks) {
                Some(fr) => {
                    if annotate {
                        vcf_record.insert_info(
                            info_key.as_bytes(),
                            vec![format!("{}", fr).as_bytes().to_vec()],
                        );
                    }
                    vcf_writer.write_record(&vcf_record)?;
                }
                None => {}
            }
        } else {
            break;
        }
    }
    Ok(())
}
