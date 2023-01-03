use super::{FileSystem, Error, ClusterValue};

pub fn create<F: FileSystem>(fs: &F) -> Result<u32, Error> {
    for cluster in 2..fs.cluster_count() {
        if let ClusterValue::Free = fs.fat_table_get(cluster)? {
            fs.fat_table_set(cluster, ClusterValue::Last)?;
            let mut offset = 0;

            while offset != fs.cluster_size() {
                let zero_data = [0u8; 32];
                fs.write(cluster, offset, &zero_data)?;
                offset += zero_data.len();
            }
            return Ok(cluster);
        }
    }

    Err(Error::NoFreeCluster)
}

pub fn extend<F: FileSystem>(fs: &F, cluster: u32) -> Result<u32, Error> {
    let new_cluster = create(fs)?;
    fs.fat_table_set(cluster, ClusterValue::Next(new_cluster))?;
    Ok(new_cluster)
}

pub fn remove<F: FileSystem>(fs: &F, cluster: u32) -> Result<(), Error> {
    let mut cluster = cluster;

    loop {
        match fs.fat_table_get(cluster)? {
            ClusterValue::Next(next_cluster) => {
                fs.fat_table_set(cluster, ClusterValue::Free)?;
                cluster = next_cluster;
            },
            ClusterValue::Last => {
                fs.fat_table_set(cluster, ClusterValue::Free)?;
                return Ok(());
            },
            ClusterValue::Free | ClusterValue::Bad => {
                return Err(Error::UnexpectedClusterValue);
            }
        }          
    }
}

pub fn truncate<F: FileSystem>(fs: &F, cluster: u32) -> Result<(), Error> {
    let mut cluster = cluster;
    let mut first = true;

    loop {
        match fs.fat_table_get(cluster)? {
            ClusterValue::Next(next_cluster) => {
                if first {
                    first = false;
                    fs.fat_table_set(cluster, ClusterValue::Last)?;
                } else {
                    fs.fat_table_set(cluster, ClusterValue::Free)?;
                }
                
                cluster = next_cluster;
            },
            ClusterValue::Last => {
                if !first {
                    fs.fat_table_set(cluster, ClusterValue::Free)?;
                }
                return Ok(());
            },
            ClusterValue::Free | ClusterValue::Bad => {
                return Err(Error::UnexpectedClusterValue);
            }
        }          
    }
}
