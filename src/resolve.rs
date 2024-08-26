use bitvec::prelude as bv;
use log::{info, warn};
use std::cmp;
use std::io::{BufReader, BufWriter, Read, Write};
use std::iter::zip;
use std::path::Path;
use vcf::{VCFError, VCFReader, VCFRecord, VCFWriter};

use crate::forge;
use crate::vcf_util::{parse_genotype, unwrap_genotype, Genotype};

struct PosRange {
    start: u64,
    end: u64,
}

/// Resolve a cluster of overlapping sites.
///
/// Overlapping sites are not necessarily conflicting with each other. For
/// example, these records are overlapping but not conflicting:
///
/// ```
/// #CHROM  POS     ID      REF     ALT     QUAL    FILTER    FORMAT  NA00001 NA00002
/// 20      14370   .       GTTT    G       29      .         GT      0|0     1|0
/// 20      14370   .       G       T       29      .         GT      0|1     1|0
/// 20      14370   .       G       A       29      .         GT      1|0     0|0
/// ```
///
/// The position in a normalised indel site indicates the the position of the
/// immediately one base before the inserted or deleted sequence. This position
/// retain the same base in alternative alleles as in the reference. Therefore,
/// the first two records above can co-occur in a sample, e.g. NA00002.
///
/// Moreover, two records are conflicting if they are overlapping and both occur
/// in a sample (i.e. they are in coupling configuration in at least one
/// sample). For exmaple, the last two records are not conflicting since there
/// is no sample that have both alleles on the same haplotype.
fn resolve_cluster(cluster: &[VCFRecord], ranks: &forge::RegSiteMap) -> Vec<usize> {
    let mut processed = bv::bitvec![0; cluster.len()];
    let mut selected = Vec::new();
    for (idx, record) in cluster.iter().enumerate() {
        let rank = forge::forge_rank(record, ranks).unwrap_or(&usize::MAX);
        info!(
            "  [{}] {}:{}\trank={}",
            idx,
            std::str::from_utf8(record.chromosome.as_slice()).unwrap(),
            record.position,
            rank
        );
    }

    let mut idx = 0;
    while idx < cluster.len() {
        if !processed[idx] {
            let record = &cluster[idx];
            let mut hi_idx = idx;
            let mut hi_rank = forge::forge_rank(record, ranks).unwrap_or(&usize::MAX);
            for (offset, other) in cluster[idx + 1..].iter().enumerate() {
                let cursor = idx + offset + 1;
                if are_conflicting(record, other) {
                    processed.set(cursor, true);
                    let other_rank = forge::forge_rank(other, ranks).unwrap_or(&usize::MAX);
                    if other_rank < hi_rank {
                        hi_rank = other_rank;
                        hi_idx = cursor;
                    }
                }
            }
            selected.push(hi_idx);
        }
        idx += 1;
    }
    selected.sort();
    info!("Selected {:?}", selected);
    selected
}

/// Get site positional range of a record relative to the reference sequence.
///
/// **NOTE**: The range is inclusive.
///
/// This range is used to determine overlapping cluster. For example, the
/// following record has a "site range" of (20, 23):
///
/// ```
/// #CHROM  POS     ID      REF     ALT     QUAL    FILTER    FORMAT  NA00001 NA00002
/// 20      14370   .       GTTT    G       29      .         GT      0|0     1|0
/// ```
fn site_ref_range(record: &VCFRecord) -> PosRange {
    let start = record.position;
    let end = start + record.reference.len() as u64 - 1;
    PosRange { start, end }
}

/// Get positional range of bases in the reference sequence that actually
/// affected by a variant.
///
/// **NOTE**: This function assumes that all variants are "normalised"; i.e. they
/// are left-aligned and right-parsimonous.
///
/// **NOTE**: The range is inclusive.
///
/// This range, together with [`are_coupled`], is used to determine conflicting
/// records in an overlapping cluster. For example, the following record has a
/// "variant range" of (21, 23), since the first base is not affected by the
/// variant:
///
/// ```
/// #CHROM  POS     ID      REF     ALT     QUAL    FILTER    FORMAT  NA00001 NA00002
/// 20      14370   .       GTTT    G       29      .         GT      0|0     1|0
/// ```
fn variant_ref_range(record: &VCFRecord) -> PosRange {
    let mut start = record.position;
    let end = start + record.reference.len() as u64 - 1;
    if start != end {
        start += 1;
    }
    PosRange { start, end }
}

/// Check whether two variants are in coupling configuration in at least one
/// sample in the cohort.
///
/// It requires phased VCF file. Otherwise, it reports any two alleles as
/// coupled.
fn are_coupled(record1: &VCFRecord, record2: &VCFRecord) -> bool {
    if record1.header() != record2.header() {
        panic!("Inconsistent VCF headers");
    }

    for sample in record1.header().samples() {
        let gt1 = unwrap_genotype(parse_genotype(record1.genotype(sample, b"GT")), sample);
        let gt2 = unwrap_genotype(parse_genotype(record2.genotype(sample, b"GT")), sample);
        match (gt1, gt2) {
            (Genotype::Missing, Genotype::Missing) => {
                warn!(
                    "Missing genotype fields for boths records for sample '{}'",
                    std::str::from_utf8(sample).unwrap()
                );
                warn!("  consider sites with missing genotypes coupled");
                return true;
            }
            (gt, Genotype::Missing) | (Genotype::Missing, gt) => {
                warn!(
                    "Missing genotype field in at least one record for sample '{}'",
                    std::str::from_utf8(sample).unwrap()
                );
                warn!("  checking heterozygosity of the other site");
                return !is_ref_hom(&gt).unwrap();
            }
            (Genotype::Phased(v1), Genotype::Phased(v2)) => {
                if zip(v1, v2).any(|x| x.0 && x.1) {
                    info!(
                        "Found two alleles in coupling state in sample '{}'",
                        std::str::from_utf8(sample).unwrap()
                    );
                    return true;
                }
            }
            (gt1, gt2) => {
                warn!(
                    "Unphased genotypes in at least one record for sample '{}'",
                    std::str::from_utf8(sample).unwrap()
                );
                warn!("  checking heterozygosity of both sites");
                return !is_ref_hom(&gt1).unwrap() && !is_ref_hom(&gt2).unwrap();
            }
        }
    }
    false
}

/// Check whether the genotype is homozygous for reference allele.
fn is_ref_hom(genotype: &Genotype) -> Option<bool> {
    match genotype {
        Genotype::Phased(v) | Genotype::Unphased(v) => Some(v.iter().all(|x| !x)),
        Genotype::Missing => None,
    }
}

/// Check whether two variants are conflicting.
///
/// This means they are overlapping in [`variant_ref_range`] and coupled.
fn are_conflicting(first: &VCFRecord, second: &VCFRecord) -> bool {
    let first_range = variant_ref_range(first);
    let second_range = variant_ref_range(second);
    is_range_overlapping(&first_range, &second_range) && are_coupled(first, second)
}

/// Write selected records from a cluster to the output stream.
///
/// **NOTE**: The selected indices must be sorted in order to preserve the order
/// of records in the original VCF file.
fn write_selected<W>(
    vcf_writer: &mut VCFWriter<BufWriter<W>>,
    cluster: &[VCFRecord],
    selected: &Vec<usize>,
) -> Result<(), VCFError>
where
    W: Write,
{
    for idx in selected {
        vcf_writer.write_record(&cluster[*idx])?;
    }
    Ok(())
}

/// Check whether two positional ranges are overlapping.
fn is_range_overlapping(r1: &PosRange, r2: &PosRange) -> bool {
    let mut left = &r1;
    let mut right = &r2;
    if right.start < left.start {
        std::mem::swap(&mut left, &mut right);
    }
    left.start <= right.start && right.start <= left.end
}

/// Merge two positional ranges
fn merge_range(r1: &PosRange, r2: &PosRange) -> PosRange {
    let start = cmp::min(r1.start, r2.start);
    let end = cmp::max(r1.end, r2.end);
    PosRange { start, end }
}

/// Resolve overlapping variants by ranking.
///
/// # Arguments
///
/// * `vcf_reader` - VCF input stream
/// * `vcf_writer` - VCF output stream
/// * `ranks_path` - FORGe ranking file path
///
/// **NOTE**: The input VCF file must be sorted by CHROM and POS and variants
/// should be normalised.
pub fn resolve<T, W, R>(
    mut vcf_writer: VCFWriter<BufWriter<W>>,
    mut vcf_reader: VCFReader<BufReader<R>>,
    ranks_path: &T,
) -> Result<(), VCFError>
where
    T: AsRef<Path>,
    W: Write,
    R: Read,
{
    let ranks = forge::load_rank(ranks_path, 1.0);
    let mut cur_record = VCFRecord::new(vcf_reader.header().clone());
    let mut pre_record = VCFRecord::new(vcf_reader.header().clone());
    let pre_fetched = vcf_reader.next_record(&mut pre_record)?;
    if pre_fetched {
        let mut pre_range = site_ref_range(&pre_record);
        let mut cluster = Vec::new();
        loop {
            let fetched = vcf_reader.next_record(&mut cur_record)?;
            if fetched {
                let mut cur_range = site_ref_range(&cur_record);
                let p_chrom = &pre_record.chromosome;
                let c_chrom = &cur_record.chromosome;
                if p_chrom == c_chrom {
                    if is_range_overlapping(&pre_range, &cur_range) {
                        if cluster.is_empty() {
                            cluster.push(pre_record.clone());
                        }
                        cluster.push(cur_record.clone());
                        pre_range = merge_range(&pre_range, &cur_range);
                        std::mem::swap(&mut pre_record, &mut cur_record);
                        continue;
                    }
                }
                if !cluster.is_empty() {
                    info!(
                        "Found a cluster of overlapping sites of size {}",
                        cluster.len()
                    );
                    let selected = resolve_cluster(&cluster, &ranks);
                    write_selected(&mut vcf_writer, &cluster, &selected)?;
                    cluster.clear();
                } else {
                    vcf_writer.write_record(&pre_record)?;
                }
                std::mem::swap(&mut pre_range, &mut cur_range);
                std::mem::swap(&mut pre_record, &mut cur_record);
            } else {
                vcf_writer.write_record(&pre_record)?;
                break;
            }
        }
    }
    Ok(())
}
