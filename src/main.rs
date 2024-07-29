#![cfg_attr(feature = "unstable", feature(test))]

use auto_enums::auto_enum;
use clap::{Parser, Subcommand};
use eyre::{Context, Result};
use itertools::Itertools;
use nodeset::{IdRangeList, NodeSet, Resolver};
use std::io;
use std::io::Read;

#[derive(Parser)]
#[command(about = "Operations on set of nodes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fold nodesets or individual nodes into a nodeset
    Fold {
        /// Nodesets to fold
        nodeset: Option<Vec<String>>,
    },
    /// List individual nodes in nodesets
    List {
        /// Nodesets to expand into a list
        nodeset: Option<Vec<String>>,
        /// Separator between nodes
        #[arg(short, default_value = " ")]
        separator: String,
    },
    /// Count nodes in nodesets
    Count {
        /// Nodesets to count
        nodeset: Option<Vec<String>>,
    },
    /// List groups of nodes
    Groups {
        /// List groups from all sources
        #[arg(short)]
        all_sources: bool,
        /// List groups from the specified source
        #[arg(short, conflicts_with("all_sources"))]
        source: Option<String>,
        /// Display group members
        #[arg(short)]
        members: bool,
        /// Display groups intersecting with provided nodesets
        nodeset: Option<Vec<String>>,
    },
    /// List group sources
    Sources {},
}

fn main() -> Result<()> {
    env_logger::init();
    Resolver::set_global(Resolver::from_config()?);
    use std::io::Write;
    let args = Cli::parse();
    match args.command {
        Commands::Fold { nodeset } => {
            let nodeset = nodeset_argument(nodeset)?;
            println!("{}", nodeset);
        }
        Commands::List { nodeset, separator } => {
            let nodeset = nodeset_argument(nodeset)?;
            let mut it = nodeset.iter();

            let mut lock = io::stdout().lock();

            if let Some(first) = it.next() {
                lock.write_all(first.as_bytes())?;
            }
            for node in it {
                lock.write_all(separator.as_bytes())?;
                lock.write_all(node.as_bytes())?;
            }

            println!();
        }
        Commands::Count { nodeset } => {
            let nodeset = nodeset_argument(nodeset)?;
            println!("{}", nodeset.len());
        }
        Commands::Groups {
            all_sources,
            members,
            source,
            nodeset,
        } => {
            let nodeset = if nodeset.is_some() {
                Some(nodeset_argument(nodeset)?)
            } else {
                None
            };
            group_cmd(all_sources, source, members, nodeset);
        }
        Commands::Sources {} => {
            let resolver = Resolver::get_global();
            for source in resolver.sources() {
                println!(
                    "{}{}",
                    source,
                    if source == resolver.default_source() {
                        " (default)"
                    } else {
                        ""
                    }
                );
            }
        }
    }

    Ok(())
}

#[auto_enum]
fn group_cmd(
    all: bool,
    default_source: Option<String>,
    display_members: bool,
    filter: Option<NodeSet>,
) {
    let resolver = Resolver::get_global();

    let all_groups;
    let groups;

    #[auto_enum(Iterator)]
    let iter = if all {
        all_groups = resolver
            .list_all_groups::<IdRangeList>()
            .collect::<Vec<_>>();
        all_groups.iter().flat_map(|(source, groups)| {
            let source = if *source == resolver.default_source() {
                None
            } else {
                Some(*source)
            };

            groups.iter().map(move |group| (source, group))
        })
    } else {
        groups = resolver.list_groups::<IdRangeList>(default_source.as_deref());
        groups
            .iter()
            .map(|group| (default_source.as_deref(), group))
    };

    let s = iter
        .filter_map(|(source, group)| {
            let mut members = resolver.resolve::<IdRangeList>(source, &group).ok()?;

            if let Some(filter) = &filter {
                members = members.intersection(filter);
                if members.is_empty() {
                    return None;
                }
            }

            let display_source = match &source {
                Some(s) => format!("{}:", s),
                None => "".to_string(),
            };
            if display_members {
                Some(format!("@{}{} {}", display_source, group, members))
            } else {
                Some(format!("@{}{}", display_source, group))
            }
        })
        .sorted()
        .join("\n");

    println!("{}", s);
}

fn nodeset_argument(ns: Option<Vec<String>>) -> Result<NodeSet> {
    let nodeset: NodeSet = match ns {
        Some(v) if v == vec!["-".to_string()] => read_stdin()?,
        Some(v) => v.join(" "),
        None => read_stdin()?,
    }
    .parse()
    .context("failed to parse nodeset")?;

    Ok(nodeset)
}

fn read_stdin() -> Result<String> {
    let mut s = String::new();
    io::stdin()
        .lock()
        .read_to_string(&mut s)
        .context("failed to read standard input")?;
    Ok(s)
}
#[cfg(all(feature = "unstable", test))]
mod benchs {
    extern crate test;
    use super::*;
    use test::{black_box, Bencher};

    fn prepare_vector_ranges(count: u32, ranges: u32) -> Vec<u32> {
        let mut res: Vec<u32> = Vec::new();
        for i in (0..ranges).rev() {
            res.append(&mut (count * i..count * (i + 1)).collect());
        }
        return res;
    }

    fn prepare_vectors(count1: u32, count2: u32) -> (Vec<u32>, Vec<u32>) {
        let mut v1: Vec<u32> = (0..count1).collect();
        let mut v2: Vec<u32> = (1..count2 + 1).collect();
        let mut rng = thread_rng();

        v1.shuffle(&mut rng);
        v2.shuffle(&mut rng);
        (v1, v2)
    }

    fn prepare_rangelists(count1: u32, count2: u32) -> (IdRangeList, IdRangeList) {
        let (v1, v2) = prepare_vectors(count1, count2);
        let mut rl1 = IdRangeList::new(v1.clone());
        let mut rl2 = IdRangeList::new(v2.clone());

        rl1.sort();
        rl2.sort();

        (rl1, rl2)
    }

    fn prepare_rangesets(count1: u32, count2: u32) -> (IdRangeTree, IdRangeTree) {
        let (v1, v2) = prepare_vectors(count1, count2);
        (IdRangeTree::new(v1.clone()), IdRangeTree::new(v2.clone()))
    }

    const DEFAULT_COUNT: u32 = 100;

    #[bench]
    fn bench_rangelist_union_homo(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangelists(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            black_box(rl1.union(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangeset_union_homo(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangesets(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            black_box(rl1.union(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangelist_symdiff_homo(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangelists(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            black_box(rl1.symmetric_difference(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangeset_symdiff_homo(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangesets(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            black_box(rl1.symmetric_difference(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangelist_difference_homo(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangelists(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            black_box(rl1.difference(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangeset_difference_homo(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangesets(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            black_box(rl1.difference(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangelist_difference_hetero(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangelists(DEFAULT_COUNT, 10);
        b.iter(|| {
            black_box(rl1.difference(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangeset_difference_hetero(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangesets(DEFAULT_COUNT, 10);
        b.iter(|| {
            black_box(rl1.difference(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangelist_intersection(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangelists(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            black_box(rl1.intersection(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangeset_intersection(b: &mut Bencher) {
        let (rl1, rl2) = prepare_rangesets(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            black_box(rl1.intersection(&rl2).sum::<u32>());
        });
    }

    #[bench]
    fn bench_rangelist_creation_shuffle(b: &mut Bencher) {
        let (v1, _) = prepare_vectors(DEFAULT_COUNT * 100, DEFAULT_COUNT * 100);
        b.iter(|| {
            let mut rl1 = IdRangeList::new(v1.clone());
            rl1.sort();
        });
    }

    #[bench]
    fn bench_rangelist_creation_sorted(b: &mut Bencher) {
        let (mut v1, _) = prepare_vectors(DEFAULT_COUNT, DEFAULT_COUNT);
        v1.sort();
        b.iter(|| {
            let mut rl1 = IdRangeList::new(v1.clone());
            rl1.sort();
        });
    }

    #[bench]
    fn bench_rangelist_creation_ranges(b: &mut Bencher) {
        let v1 = prepare_vector_ranges(100, 10);
        b.iter(|| {
            let mut rl1 = IdRangeList::new(v1.clone());
            rl1.sort();
        });
    }

    #[bench]
    fn bench_rangeset_creation(b: &mut Bencher) {
        let (v1, _) = prepare_vectors(DEFAULT_COUNT, DEFAULT_COUNT);
        b.iter(|| {
            let _rs1 = IdRangeTree::new(v1.clone());
        });
    }

    #[bench]
    fn bench_rangeset_creation_sorted(b: &mut Bencher) {
        let (mut v1, _) = prepare_vectors(DEFAULT_COUNT, DEFAULT_COUNT);
        v1.sort();
        b.iter(|| {
            let _rs1 = IdRangeTree::new(v1.clone());
        });
    }

    #[bench]
    fn bench_rangeset_creation_ranges(b: &mut Bencher) {
        let v1 = prepare_vector_ranges(100, 10);
        b.iter(|| {
            let _rs1 = IdRangeTree::new(v1.clone());
        });
    }

    #[bench]
    fn bench_idset_intersection(b: &mut Bencher) {
        let mut id1: IdSet<IdRangeList> = IdSet::new();
        let mut id2: IdSet<IdRangeList> = IdSet::new();

        id1.push("node[0-1000000]");
        id2.push("node[1-1000001]");

        b.iter(|| {
            let _rs1 = id1.intersection(&id2);
        });
    }

    #[bench]
    fn bench_idset_intersection_set(b: &mut Bencher) {
        let mut id1: IdSet<IdRangeTree> = IdSet::new();
        let mut id2: IdSet<IdRangeTree> = IdSet::new();

        id1.push("node[0-1000000]");
        id2.push("node[1-1000001]");

        b.iter(|| {
            let _rs1 = id1.intersection(&id2);
        });
    }

    #[bench]
    fn bench_idset_print(b: &mut Bencher) {
        let mut id1: IdSet<IdRangeList> = IdSet::new();

        id1.push("node[0-10000000]");

        b.iter(|| {
            let _rs1 = id1.to_string();
        });
    }

    #[bench]
    fn bench_idset_split(b: &mut Bencher) {
        b.iter(|| {
            let mut id1: IdSet<IdRangeList> = IdSet::new();
            id1.push("node[0-100000]");
            id1.full_split();
        });
    }

    #[bench]
    fn bench_idset_split_set(b: &mut Bencher) {
        b.iter(|| {
            let mut id1: IdSet<IdRangeTree> = IdSet::new();
            id1.push("node[0-100000]");
            id1.full_split();
        });
    }

    #[bench]
    fn bench_idset_merge(b: &mut Bencher) {
        b.iter(|| {
            let mut id1: IdSet<IdRangeTree> = IdSet::new();
            id1.push("node[0-100000]");
            id1.full_split();
            id1.merge();
        });
    }
}
