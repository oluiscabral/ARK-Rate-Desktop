use std::{
    fs::{create_dir_all, read_dir, DirEntry, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    entities::{pair::Pair, pair_group::PairGroup},
    implementations::data_access::file_system::file_system_pair::FileSystemPair,
    interactors::view_pair_groups::ViewPairGroupsDataAccess,
    Error,
};

use super::file_system_pair_group::FileSystemPairGroup;

const PAIRS_DIR_NAME: &str = "pairs";
const PAIR_GROUPS_DIR_NAME: &str = "pair_groups";

pub struct FileSystemDataAccess {
    pub root: PathBuf,
}

impl ViewPairGroupsDataAccess for FileSystemDataAccess {
    async fn fetch_pair_groups(&mut self) -> Result<Vec<PairGroup>, Error> {
        let mut pair_groups: Vec<PairGroup> = vec![];
        let entries = get_dir_entries(&self.root, PAIR_GROUPS_DIR_NAME)?;
        for entry in entries {
            let file_name = entry.file_name();
            if let Some(id) = file_name.to_str() {
                let pair_group = read_pair_group(&self.root, id)?;
                pair_groups.push(pair_group);
            }
        }
        return Ok(pair_groups);
    }

    async fn update_pair_group(&mut self, pair_group: &PairGroup) -> Result<(), Error> {
        let dir = ensure_dir(&self.root, PAIR_GROUPS_DIR_NAME)?;
        let path = dir.join(&pair_group.id);
        if !path.exists() {
            return Err(Error {
                message: String::from("Pair group to update does not exist!"),
            });
        }
        write_pair_group(&self.root, pair_group)?;
        return Ok(());
    }
}

fn get_dir_entries(root: &Path, name: &str) -> Result<Vec<DirEntry>, Error> {
    let mut dir_entries: Vec<DirEntry> = vec![];
    let dir = ensure_dir(root, name)?;
    let dir_entry_results = read_dir(&dir).map_err(|e| Error {
        message: e.to_string(),
    })?;
    for dir_entry_result in dir_entry_results {
        let dir_entry = dir_entry_result.map_err(|e| Error {
            message: e.to_string(),
        })?;
        dir_entries.push(dir_entry);
    }
    return Ok(dir_entries);
}

fn ensure_dir(root: &Path, name: &str) -> Result<PathBuf, Error> {
    let dir = root.join(name);
    create_dir_all(&dir).expect("Could not create database directory!");
    return Ok(dir);
}

fn read_pair_group(root: &Path, id: &str) -> Result<PairGroup, Error> {
    let dir = ensure_dir(root, PAIR_GROUPS_DIR_NAME)?;
    let path = dir.join(id);
    let fs_pair_group = create_object_from_file::<FileSystemPairGroup>(&path)?;
    let mut pair_group = PairGroup {
        id: fs_pair_group.id.clone(),
        pairs: vec![],
        is_pinned: fs_pair_group.is_pinned,
        created_at: fs_pair_group.created_at.clone(),
        updated_at: fs_pair_group.updated_at.clone(),
    };
    for pair_id in &fs_pair_group.pairs {
        let pair = read_pair(root, &pair_id)?;
        pair_group.pairs.push(pair);
    }
    return Ok(pair_group);
}

fn create_object_from_file<T>(path: &Path) -> Result<T, Error>
where
    T: for<'a> Deserialize<'a>,
{
    let mut file = File::open(path).map_err(|e| Error {
        message: e.to_string(),
    })?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| Error {
        message: e.to_string(),
    })?;
    let object = serde_json::from_str::<T>(&contents).map_err(|e| Error {
        message: e.to_string(),
    })?;
    return Ok(object);
}

fn read_pair(root: &Path, id: &str) -> Result<Pair, Error> {
    let dir = ensure_dir(root, PAIRS_DIR_NAME)?;
    let path = dir.join(id);
    let fs_pair = create_object_from_file::<FileSystemPair>(&path)?;
    return Ok(Pair {
        id: fs_pair.id.clone(),
        base: fs_pair.base.clone(),
        value: fs_pair.value.clone(),
        comparison: fs_pair.comparison.clone(),
        created_at: fs_pair.created_at.clone(),
        updated_at: fs_pair.updated_at.clone(),
    });
}

fn write_pair_group(root: &Path, pair_group: &PairGroup) -> Result<(), Error> {
    for pair in &pair_group.pairs {
        write_pair(root, pair)?;
    }
    let dir = ensure_dir(root, PAIR_GROUPS_DIR_NAME)?;
    let path = dir.join(&pair_group.id);
    write_object_file(
        &path,
        &FileSystemPairGroup {
            id: pair_group.id.clone(),
            is_pinned: pair_group.is_pinned,
            pairs: pair_group.pairs.iter().map(|p| p.id.clone()).collect(),
            created_at: pair_group.created_at.clone(),
            updated_at: pair_group.updated_at.clone(),
        },
    )?;
    return Ok(());
}

fn write_pair(root: &Path, pair: &Pair) -> Result<(), Error> {
    let dir = ensure_dir(root, PAIRS_DIR_NAME)?;
    let path = dir.join(&pair.id);
    write_object_file(
        &path,
        &FileSystemPair {
            id: pair.id.clone(),
            base: pair.base.clone(),
            value: pair.value.clone(),
            comparison: pair.comparison.clone(),
            created_at: pair.created_at.clone(),
            updated_at: pair.updated_at.clone(),
        },
    )?;
    return Ok(());
}

fn write_object_file<T>(path: &Path, object: &T) -> Result<(), Error>
where
    T: for<'a> Serialize,
{
    let object_contents = serde_json::to_string(object).map_err(|e| Error {
        message: e.to_string(),
    })?;
    File::create(path)
        .and_then(|mut file| file.write_all(object_contents.as_bytes()))
        .map_err(|e| Error {
            message: e.to_string(),
        })?;
    return Ok(());
}

#[cfg(test)]
mod tests {
    use crate::entities::pair::Pair;

    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fetch_pair_groups() {
        /*
            Unit test expectations:

            - The fetched number of pair groups equals the number of written pair groups.
            - Each fetched pair group matches the corresponding example pair group.
        */
        let temp_dir = tempdir().unwrap();
        let root = temp_dir.path();

        let example_pairs: Vec<Pair> = vec![
            Pair {
                id: "p1".to_string(),
                value: 1.0,
                base: "USD".to_string(),
                comparison: "BTC".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            },
            Pair {
                id: "p2".to_string(),
                value: 2.0,
                base: "USD".to_string(),
                comparison: "ETH".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            },
            Pair {
                id: "p3".to_string(),
                value: 3.0,
                base: "USD".to_string(),
                comparison: "BRL".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            },
        ];

        let example_pair_groups = vec![
            PairGroup {
                id: "pg1".to_string(),
                is_pinned: true,
                pairs: vec![example_pairs[0].clone(), example_pairs[1].clone()],
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            },
            PairGroup {
                id: "pg2".to_string(),
                is_pinned: false,
                pairs: vec![example_pairs[2].clone()],
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            },
        ];

        for example_pair_group in &example_pair_groups {
            write_pair_group(&root, example_pair_group).unwrap();
        }

        let mut data_access: FileSystemDataAccess = FileSystemDataAccess {
            root: root.to_path_buf(),
        };

        let pair_groups = data_access.fetch_pair_groups().await.unwrap();
        assert_eq!(pair_groups.len(), 2);
        assert_eq!(pair_groups[0], example_pair_groups[0]);
        assert_eq!(pair_groups[1], example_pair_groups[1]);

        std::fs::remove_dir_all(root).expect("Failed to clear test temp directory");
    }

    #[tokio::test]
    async fn test_update_pair_group() {
        /*
            Unit test expectations:

            - The previously written pair group should be replaced by the updated pair group.
        */
        let temp_dir = tempdir().unwrap();
        let root = temp_dir.path();

        let example_pairs: Vec<Pair> = vec![
            Pair {
                id: "p1".to_string(),
                value: 1.0,
                base: "USD".to_string(),
                comparison: "BTC".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            },
            Pair {
                id: "p2".to_string(),
                value: 2.0,
                base: "USD".to_string(),
                comparison: "ETH".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            },
            Pair {
                id: "p3".to_string(),
                value: 3.0,
                base: "USD".to_string(),
                comparison: "BRL".to_string(),
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            },
        ];

        let original_pair_group = PairGroup {
            id: "pg1".to_string(),
            is_pinned: false,
            pairs: vec![example_pairs[0].clone(), example_pairs[1].clone()],
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        write_pair_group(&root, &original_pair_group).unwrap();

        let updated_pair_group = PairGroup {
            id: "pg1".to_string(),
            is_pinned: true,
            pairs: vec![
                example_pairs[0].clone(),
                example_pairs[1].clone(),
                example_pairs[2].clone(),
            ],
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        let mut data_access: FileSystemDataAccess = FileSystemDataAccess {
            root: root.to_path_buf(),
        };

        data_access
            .update_pair_group(&updated_pair_group)
            .await
            .unwrap();

        let stored_pair_group = read_pair_group(root, "pg1").unwrap();
        assert_eq!(stored_pair_group, updated_pair_group);

        std::fs::remove_dir_all(root).expect("Failed to clear test temp directory");
    }
}
