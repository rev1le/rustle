//! Aggregate metadata by directory

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use crate::models::{DirAggregate, FileMeta};

/// Group files by directory with size/count/per-ext breakdown
pub fn aggregate(files: &[FileMeta]) -> Vec<DirAggregate> {
    // dir path → (total_size, file_count, ext → size)
    let mut dirs: BTreeMap<PathBuf, (u64, usize, HashMap<&str, u64>)> = BTreeMap::new();

    for file in files {
        // borrows from `files`, no clone needed
        let ext_key: &str = file.ext.as_deref().unwrap_or("(no ext)");

        // skip the file's own path; ancestors() then yields parents up to root
        let mut ancestors = file.path.ancestors();
        ancestors.next();

        for dir in ancestors {
            // skip empty component (relative paths)
            if dir.as_os_str().is_empty() {
                continue;
            }
            let entry = dirs.entry(dir.to_path_buf()).or_default();
            entry.0 += file.size;
            entry.1 += 1;
            *entry.2.entry(ext_key).or_insert(0) += file.size;
        }
    }

    let mut result: Vec<DirAggregate> = dirs
        .into_iter()
        .map(|(path, (total_size, file_count, ext_map))| DirAggregate {
            path,
            total_size,
            file_count,
            by_extension: {
                let mut v: Vec<(String, u64)> = ext_map
                    .into_iter()
                    .map(|(ext, size)| (ext.to_string(), size))
                    .collect();
                v.sort_by_key(|(_ext, size)| std::cmp::Reverse(*size));
                v
            },
        })
        .collect();

    result.sort_by_key(|dir| std::cmp::Reverse(dir.total_size));
    result
}

/// Top-N largest files, descending
pub fn top_largest(files: &[FileMeta], n: usize) -> Vec<FileMeta> {
    let mut sorted: Vec<&FileMeta> = files.iter().collect();
    sorted.sort_by_key(|file| std::cmp::Reverse(file.size));
    sorted.into_iter().take(n).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::human_readable_size;
    use chrono::Utc;
    use std::path::Path;

    fn file(path: &str, size: u64, ext: Option<&str>) -> FileMeta {
        FileMeta {
            path: PathBuf::from(path),
            size,
            ext: ext.map(|e| e.to_string()),
            mtime: Utc::now(),
        }
    }

    fn find<'a>(agg: &'a [DirAggregate], path: &str) -> &'a DirAggregate {
        agg.iter()
            .find(|d| d.path == Path::new(path))
            .unwrap_or_else(|| panic!("directory {path:?} not found in aggregate"))
    }

    mod aggregate {
        use super::*;

        #[test]
        fn counts_files_recursively_into_ancestors() {
            let files = vec![
                file("/proj/a.txt", 100, Some("txt")),
                file("/proj/sub/b.txt", 200, Some("txt")),
                file("/proj/sub/c.log", 400, Some("log")),
            ];
            let agg = aggregate(&files);

            let proj = find(&agg, "/proj");
            assert_eq!(proj.total_size, 700);
            assert_eq!(proj.file_count, 3);
        }

        #[test]
        fn sums_only_descendants_into_subdir() {
            let files = vec![
                file("/proj/a.txt", 100, Some("txt")),
                file("/proj/sub/b.txt", 200, Some("txt")),
                file("/proj/sub/c.log", 400, Some("log")),
            ];
            let agg = aggregate(&files);

            let sub = find(&agg, "/proj/sub");
            assert_eq!((sub.total_size, sub.file_count), (600, 2));
        }

        #[test]
        fn breaks_down_extensions_sorted_by_size_desc() {
            let files = vec![
                file("/proj/sub/b.txt", 200, Some("txt")),
                file("/proj/sub/c.log", 400, Some("log")),
            ];
            let agg = aggregate(&files);
            let sub = find(&agg, "/proj/sub");

            assert_eq!(sub.by_extension[0], ("log".to_string(), 400));
            assert_eq!(sub.by_extension[1], ("txt".to_string(), 200));
        }

        #[test]
        fn groups_files_without_extension_under_placeholder() {
            let agg = aggregate(&[file("/proj/Makefile", 50, None)]);
            let proj = find(&agg, "/proj");

            assert_eq!(proj.total_size, 50);
            assert_eq!(proj.by_extension[0].0, "(no ext)");
        }

        #[test]
        fn orders_sibling_dirs_by_size_desc() {
            // root accumulates all; check sibling order under /tree
            let agg = aggregate(&[
                file("/tree/big/x", 1000, Some("bin")),
                file("/tree/small/y", 10, Some("txt")),
            ]);

            let i_big = agg
                .iter()
                .position(|d| d.path == Path::new("/tree/big"))
                .unwrap();
            let i_small = agg
                .iter()
                .position(|d| d.path == Path::new("/tree/small"))
                .unwrap();
            assert!(i_big < i_small, "/tree/big must rank above /tree/small");
        }

        #[test]
        fn root_is_the_largest_dir() {
            let agg = aggregate(&[
                file("/tree/big/x", 1000, Some("bin")),
                file("/tree/small/y", 10, Some("txt")),
            ]);
            assert_eq!(agg[0].total_size, 1010);
        }

        #[test]
        fn returns_empty_for_empty_input() {
            assert!(aggregate(&[]).is_empty());
        }
    }

    mod top_largest {
        use super::*;

        #[test]
        fn returns_n_biggest_in_descending_order() {
            let files = vec![
                file("/a", 10, Some("txt")),
                file("/b", 50, Some("log")),
                file("/c", 30, Some("raw")),
                file("/d", 50, Some("bin")),
            ];
            let top = top_largest(&files, 2);

            assert_eq!(top.len(), 2);
            assert_eq!((top[0].size, top[1].size), (50, 50));
        }

        #[test]
        fn returns_all_when_n_exceeds_len() {
            let files = vec![file("/a", 1, None), file("/b", 2, None)];
            let top = top_largest(&files, 10);

            assert_eq!(top.len(), 2);
            assert_eq!(top[0].size, 2);
        }

        #[test]
        fn returns_empty_for_empty_input() {
            assert!(top_largest(&[], 5).is_empty());
        }
    }

    #[test]
    fn aggregate_size_formats_via_human_readable() {
        // smoke: aggregation result must format without panicking
        let agg = aggregate(&[file("/proj/x", 1_048_576, Some("iso"))]);
        assert_eq!(human_readable_size(agg[0].total_size), "1.00 MiB");
    }
}
