use log::{error, warn};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use vcf::VCFRecord;

pub type Region = Vec<u8>;
pub type SiteMap = HashMap<u64, usize>;
pub type RegSiteMap = HashMap<Region, SiteMap>;

/// Prettify and truncate a token for logging
///
/// # Arguments
///
/// `s` - string to be prettified and truncated
/// `n` - maximum length
fn pretty_trunc(s: &str, n: usize) -> String {
    let prettify = |x: &str| x.replace("\n", "⏎  ");
    if s.len() <= n {
        prettify(s).to_string()
    } else {
        prettify(&s[..n]).to_string() + "..."
    }
}

/// Parse an entry in the FORGe ranking file
pub fn parse_id(id: &str) -> Option<(Region, u64)> {
    let tokens: Vec<&str> = id.split(',').collect();
    if tokens.len() != 2 {
        return None;
    }
    match (tokens[0], tokens[1].parse::<u64>()) {
        (region, Ok(pos)) => {
            if region.is_ascii() {
                Some((region.as_bytes().to_vec(), pos))
            } else {
                warn!("Non-ASCII characters in the region name '{}'", region);
                None
            }
        }
        _ => None,
    }
}

/// Get FORGe rank of a VCF record
pub fn forge_rank<'a>(record: &VCFRecord, ranks: &'a RegSiteMap) -> Option<&'a usize> {
    match ranks.get(&record.chromosome) {
        Some(sitemap) => sitemap.get(&record.position),
        None => None,
    }
}

/// Load ranks file into a `RegSiteMap` instance
///
/// # Arguments
///
/// `path` - path to FORGe ranks file (output by FORGe's `rank.py`)
/// `top` - only load first (100*`top`)% of variants in the file
pub fn load_rank<T>(path: T, top: f64) -> RegSiteMap
where
    T: AsRef<std::path::Path>,
{
    let mut file = File::open(path).expect("FORGe rank file not found");
    let reader = BufReader::new(&file);

    let nof_records = reader.split(b'\t').count();

    file.seek(SeekFrom::Start(0))
        .expect("Cannot seek to start of file");
    let reader = BufReader::new(&file);

    let n = (top * nof_records as f64) as usize;
    let mut smap = RegSiteMap::new();
    let mut r: usize = 1;
    let mut i: usize = 0;
    for item in reader.split(b'\t') {
        match item {
            Ok(item) => {
                let rec = String::from_utf8_lossy(&item);
                match parse_id(&rec.trim_end()) {
                    Some((region, pos)) => {
                        let entry = smap.entry(region).or_insert(HashMap::new()).entry(pos);

                        match entry {
                            Entry::Occupied(_) => {
                                warn!(
                                    "Duplicated FORGe record (rank: {}): '{}'",
                                    r,
                                    pretty_trunc(&rec, 30)
                                );
                            }
                            Entry::Vacant(v) => {
                                v.insert(r);
                                i += 1;
                                if i == n {
                                    break;
                                }
                            }
                        }
                    }

                    None => {
                        let message = format!(
                            "Invalid FORGe record (rank: {}): '{}'",
                            r,
                            pretty_trunc(&rec, 30)
                        );
                        if nof_records < 2 {
                            error!("{}", message);
                            std::process::exit(1);
                        } else {
                            warn!("{}", message);
                        }
                    }
                }
                r += 1;
            }

            Err(_) => {
                error!("Error reading an item from the rank file");
            }
        }
    }

    if i < n {
        warn!("Not enough distinct records in the rank file");
    }

    smap
}
