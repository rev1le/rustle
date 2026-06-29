//! Агрегация собранных метаданных по директориям (Этап 1 MVP).

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::models::{DirAggregate, FileMeta};

/// Сгруппировать файлы по родительским директориям и посчитать
/// суммарный размер, количество и распределение по расширениям.
///
/// Каждая директория учитывается рекурсивно (в стиле `du`): файл
/// `/proj/sub/a.txt` увеличивает итоги для `/proj/sub`, `/proj` и корня.
/// Результат отсортирован по убыванию суммарного размера.
pub fn aggregate(files: &[FileMeta]) -> Vec<DirAggregate> {
    // dir path → (total_size, file_count, ext → size)
    let mut dirs: BTreeMap<PathBuf, (u64, usize, BTreeMap<String, u64>)> = BTreeMap::new();

    for file in files {
        let ext_key = file.ext.clone().unwrap_or_else(|| "(без ext)".to_string());

        // ancestors() сначала отдаёт сам путь файла — пропускаем его,
        // далее идут родительские каталоги вплоть до корня.
        let mut ancestors = file.path.ancestors();
        let _ = ancestors.next();

        for dir in ancestors {
            // Пропускаем пустой компонент (возникает у относительных путей).
            if dir.as_os_str().is_empty() {
                continue;
            }
            let entry = dirs.entry(dir.to_path_buf()).or_default();
            entry.0 += file.size;
            entry.1 += 1;
            *entry.2.entry(ext_key.clone()).or_insert(0) += file.size;
        }
    }

    let mut result: Vec<DirAggregate> = dirs
        .into_iter()
        .map(|(path, (total_size, file_count, ext_map))| DirAggregate {
            path,
            total_size,
            file_count,
            by_extension: {
                let mut v: Vec<(String, u64)> = ext_map.into_iter().collect();
                v.sort_by(|a, b| b.1.cmp(&a.1));
                v
            },
        })
        .collect();

    result.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    result
}

/// Выбрать топ-N самых больших файлов (в порядке убывания размера).
pub fn top_largest(files: &[FileMeta], n: usize) -> Vec<FileMeta> {
    let mut sorted: Vec<&FileMeta> = files.iter().collect();
    sorted.sort_by(|a, b| b.size.cmp(&a.size));
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
            .unwrap_or_else(|| panic!("директория {path:?} не найдена в агрегате"))
    }

    #[test]
    fn aggregate_recursive_sizes_and_counts() {
        let files = vec![
            file("/proj/a.txt", 100, Some("txt")),
            file("/proj/sub/b.txt", 200, Some("txt")),
            file("/proj/sub/c.log", 400, Some("log")),
        ];
        let agg = aggregate(&files);

        // /proj/sub содержит b.txt + c.log.
        let sub = find(&agg, "/proj/sub");
        assert_eq!(sub.total_size, 600);
        assert_eq!(sub.file_count, 2);

        // /proj накапливает все три файла.
        let proj = find(&agg, "/proj");
        assert_eq!(proj.total_size, 700);
        assert_eq!(proj.file_count, 3);
    }

    #[test]
    fn aggregate_breaks_down_by_extension_sorted_desc() {
        let files = vec![
            file("/proj/sub/b.txt", 200, Some("txt")),
            file("/proj/sub/c.log", 400, Some("log")),
        ];
        let agg = aggregate(&files);
        let sub = find(&agg, "/proj/sub");

        // log (400) должен идти раньше txt (200).
        assert_eq!(sub.by_extension[0], ("log".to_string(), 400));
        assert_eq!(sub.by_extension[1], ("txt".to_string(), 200));
    }

    #[test]
    fn aggregate_handles_missing_extension() {
        let files = vec![file("/proj/Makefile", 50, None)];
        let agg = aggregate(&files);
        let proj = find(&agg, "/proj");
        assert_eq!(proj.total_size, 50);
        assert_eq!(proj.by_extension[0].0, "(без ext)");
    }

    #[test]
    fn aggregate_is_sorted_by_size_desc() {
        // Корень аккумулирует всё, поэтому проверяем порядок сестринских
        // каталогов под общим родителем /tree.
        let files = vec![
            file("/tree/big/x", 1000, Some("bin")),
            file("/tree/small/y", 10, Some("txt")),
        ];
        let agg = aggregate(&files);

        let i_big = agg
            .iter()
            .position(|d| d.path == Path::new("/tree/big"))
            .unwrap();
        let i_small = agg
            .iter()
            .position(|d| d.path == Path::new("/tree/small"))
            .unwrap();
        // Более тяжёлый сестринский каталог должен идти раньше.
        assert!(i_big < i_small, "/tree/big должен быть выше /tree/small");

        // А корень — самым тяжёлым (содержит оба файла).
        assert_eq!(agg[0].total_size, 1010);
    }

    #[test]
    fn aggregate_empty_input() {
        assert!(aggregate(&[]).is_empty());
    }

    #[test]
    fn top_largest_returns_n_biggest_in_order() {
        let files = vec![
            file("/a", 10, Some("txt")),
            file("/b", 50, Some("log")),
            file("/c", 30, Some("raw")),
            file("/d", 50, Some("bin")),
        ];
        let top = top_largest(&files, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].size, 50);
        assert_eq!(top[1].size, 50);
    }

    #[test]
    fn top_largest_n_bigger_than_len_returns_all() {
        let files = vec![file("/a", 1, None), file("/b", 2, None)];
        let top = top_largest(&files, 10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].size, 2);
    }

    #[test]
    fn top_largest_empty() {
        assert!(top_largest(&[], 5).is_empty());
    }

    #[test]
    fn human_readable_size_via_aggregate_smoke() {
        // Дымовой тест: результат агрегации должен форматироваться без паник.
        let agg = aggregate(&[file("/proj/x", 1_048_576, Some("iso"))]);
        assert_eq!(human_readable_size(agg[0].total_size), "1.00 MiB");
    }
}
