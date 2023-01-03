use super::block_device::BlockDevice;
use super::{ClusterValue, FatType, FileSystem, Error};

pub struct Fs<D: BlockDevice> {
    dev: D,
    root_cluster: u32,
    first_data_sector: u32,
    first_fat_table_sector: u32,
    sectors_in_cluster: u32,
    sector_size: u32,
    clusters_count: u32,
    fats_count: u32,
    fat_size_in_sectors: u32,
    fat_type: FatType,
    root_dir_sectors: u32,
}

impl <D: BlockDevice> FileSystem for Fs<D> {
    fn root_cluster(&self) -> u32 {
        self.root_cluster
    }

    fn cluster_size(&self) -> usize {
        (self.sectors_in_cluster * self.sector_size) as usize
    }

    fn cluster_count(&self) -> u32 {
        self.clusters_count
    }

    fn read(&self, cluster: u32, offset: usize, buf: &mut [u8]) -> Result<usize, Error> {
        let sector = self.cluster_to_sector(cluster)?;
        self.dev.read(sector, offset % self.sector_size as usize, buf).or(Err(Error::DeviceIO))
    }

    fn write(&self, cluster: u32, offset: usize, buf: &[u8]) -> Result<usize, Error> {
        let sector = self.cluster_to_sector(cluster)?;
        self.dev.write(sector, offset % self.sector_size as usize, buf).or(Err(Error::DeviceIO))
    }

    fn fat_table_get(&self, cluster: u32) -> Result<ClusterValue, Error> {
        match self.fat_type {
            FatType::Fat12 => self.fat12_table_get(cluster),
            FatType::Fat16 => self.fat16_table_get(cluster),
            FatType::Fat32 => self.fat32_table_get(cluster),
        }
    }

    fn fat_table_set(&self, cluster: u32, value: ClusterValue) -> Result<(), Error> {
        match self.fat_type {
            FatType::Fat12 => self.fat12_table_set(cluster, value),
            FatType::Fat16 => self.fat16_table_set(cluster, value),
            FatType::Fat32 => self.fat32_table_set(cluster, value),
        }
    }

    fn flush(&self) -> Result<(), Error> {
        self.dev.flush().or(Err(Error::DeviceIO))
    }
}


impl <D: BlockDevice> Fs<D> {
    fn fat12_table_get(&self, cluster: u32) -> Result<ClusterValue, Error> {
        let sector = self.first_fat_table_sector + (cluster + (cluster / 2)) / self.sector_size;
        let offset = ((cluster + (cluster / 2)) % self.sector_size) as usize;
        let mut raw = [0u8; 2];
        self.dev.read(sector, offset, &mut raw).or(Err(Error::DeviceIO))?;
        let val = u16::from_le_bytes(raw);

        println!("{:2X} {:2X}", raw[0], raw[1]);

        let raw_value = if cluster & 1 == 0 {
            (val & 0x0FFF) as u32
        } else {
            (val >> 4) as u32
        };

        Ok(match raw_value {
            0 => ClusterValue::Free,
            0xFF7 => ClusterValue::Bad,
            0xFF8..=0xFFF => ClusterValue::Last,
            value => ClusterValue::Next(value),
        })
    }

    fn fat16_table_get(&self, cluster: u32) -> Result<ClusterValue, Error> {
        let sector = self.first_fat_table_sector + (2 * cluster / self.sector_size);
        let offset = (2 * cluster % self.sector_size) as usize;
        let mut raw = [0u8; 2];
        self.dev.read(sector, offset, &mut raw).or(Err(Error::DeviceIO))?;
        
        Ok(match u16::from_le_bytes(raw) {
            0 => ClusterValue::Free,
            0xFFF7 => ClusterValue::Bad,
            0xFFF8..=0xFFFF => ClusterValue::Last,
            value => ClusterValue::Next(value as u32),
        })
    }

    fn fat32_table_get(&self, cluster: u32) -> Result<ClusterValue, Error> {
        let mut raw = [0u8; 4];
        let sector = self.first_fat_table_sector + (4 * cluster / self.sector_size);
        let offset = (4 * cluster % self.sector_size) as usize;
        self.dev.read(sector, offset, &mut raw).or(Err(Error::DeviceIO))?;
        
        Ok(match u32::from_le_bytes(raw) & 0x0FFF_FFFF {
            0 => ClusterValue::Free,
            0x0FFF_FFF7 => ClusterValue::Bad,
            0x0FFF_FFF8..=core::u32::MAX => ClusterValue::Last,
            value => ClusterValue::Next(value),
        })
    }

    fn fat32_table_set(&self, cluster: u32, value: ClusterValue) -> Result<(), Error> {
        let n = match value {
            ClusterValue::Free => 0x00000000,
            ClusterValue::Bad => 0x0FFFFFF7,
            ClusterValue::Last => 0x0FFFFFF8,
            ClusterValue::Next(x) => x,
        };

        let mut sector = self.first_fat_table_sector + (4 * cluster / self.sector_size);
        let offset = (4 * cluster % self.sector_size) as usize;

        for _ in 0..self.fats_count {
            self.dev.write(sector, offset, &n.to_le_bytes()).or(Err(Error::DeviceIO))?;
            sector += self.fat_size_in_sectors;
        }
        
        Ok(())
    }

    fn fat16_table_set(&self, cluster: u32, value: ClusterValue) -> Result<(), Error> {
        let n = match value {
            ClusterValue::Free => 0x0000,
            ClusterValue::Bad => 0xFFF7,
            ClusterValue::Last => 0xFFF8,
            ClusterValue::Next(x) => x,
        } as u16;

        let mut sector = self.first_fat_table_sector + (2 * cluster / self.sector_size);
        let offset = (2 * cluster % self.sector_size) as usize;

        for _ in 0..self.fats_count {
            self.dev.write(sector, offset, &n.to_le_bytes()).or(Err(Error::DeviceIO))?;
            sector += self.fat_size_in_sectors;
        }
        
        Ok(())
    }

    fn fat12_table_set(&self, cluster: u32, value: ClusterValue) -> Result<(), Error> {
        let raw_value = match value {
            ClusterValue::Next(n) => n & 0xFFF,
            ClusterValue::Last => 0xFF8,
            ClusterValue::Free => 0,
            ClusterValue::Bad => 0xFF7,
        };

        let mut sector = self.first_fat_table_sector + (cluster + (cluster / 2)) / self.sector_size;
        let offset = ((cluster + (cluster / 2)) % self.sector_size) as usize;
        let mut raw = [0u8; 2];
        self.dev.read(sector, offset, &mut raw).or(Err(Error::DeviceIO))?;

        if cluster & 1 == 0 {
            raw[0] = raw_value as u8;
            raw[1] = (raw[1] & 0x0f) | (((raw_value >> 8) & 0x0f) as u8);
        } else {
            raw[0] = (raw[0] & 0xf0) | (((raw_value & 0x0f) << 4) as u8);
            raw[1] = (raw_value >> 8) as u8;
        }
        
        for _ in 0..self.fats_count {
            self.dev.write(sector, offset, &raw).or(Err(Error::DeviceIO))?;
            sector += self.fat_size_in_sectors;
        }
        
        Ok(())
    }

    pub fn mount(dev: D) -> Result<Self, Error> {
        let mut boot = [0u8; 512];
        dev.read(0, 0, &mut boot).or(Err(Error::DeviceIO)).or(Err(Error::DeviceIO))?;

        let sector_size = u16::from_le_bytes([boot[11], boot[12]]) as u32;
        println!("sector size: {}", sector_size);

        let sectors_in_cluster = boot[13] as u32;
        println!("sectors in cluster: {}", sectors_in_cluster);

        let reserved_sectors_count = u16::from_le_bytes([boot[14], boot[15]]) as u32;
        println!("reserved sectors count: {}", reserved_sectors_count);

        let fats_count = boot[16] as u32;
        println!("fats count: {}", fats_count);

        let root_entries_count = u16::from_le_bytes([boot[17], boot[18]]) as u32;
        println!("root entries count: {}", root_entries_count);

        let sectors_count_16 = u16::from_le_bytes([boot[19], boot[20]]) as u32;
        println!("sectors count (FAT12 and FAT16): {}", sectors_count_16);

        let fat_size_in_sectors_16 = u16::from_le_bytes([boot[22], boot[23]]) as u32;
        println!("fat_size_in_sectors (FAT12 and FAT16): {}", fat_size_in_sectors_16);

        let sectors_count_32 = u32::from_le_bytes([boot[32], boot[33], boot[34], boot[35]]);
        println!("sectors count (FAT32): {}", sectors_count_32);

        let fat_size_in_sectors_32 = u32::from_le_bytes([boot[36], boot[37], boot[38], boot[39]]);
        println!("fat_size_in_sectors (FAT32): {}", fat_size_in_sectors_32);

        let root_dir_sectors = ((root_entries_count * 32) + (sector_size - 1)) / sector_size;
        println!("root_dir_sectors: {}", root_dir_sectors);

        let fat_size_in_sectors =  if fat_size_in_sectors_16 != 0{
            fat_size_in_sectors_16
        } else {
            fat_size_in_sectors_32
        };

        let sectors_count = if sectors_count_16 != 0{
            sectors_count_16
        } else {
            sectors_count_32
        };

        let data_sectors_count = sectors_count - (reserved_sectors_count + (fats_count * fat_size_in_sectors) + root_dir_sectors);
        let clusters_count = data_sectors_count / sectors_in_cluster;
        let first_data_sector = reserved_sectors_count + (fats_count * fat_size_in_sectors) + root_dir_sectors;
        let first_fat_table_sector = reserved_sectors_count;
        let fat_type = determine_fat_type_by_clusters_count(clusters_count);
        println!("{:?}", fat_type);

        let root_cluster = if fat_type == FatType::Fat32 {
            u32::from_le_bytes([boot[44], boot[45], boot[46], boot[47]])
        } else {
            0
        };

        Ok(Self {
            dev: dev,
            root_cluster,
            first_data_sector,
            first_fat_table_sector,
            sectors_in_cluster,
            sector_size,
            clusters_count,
            fats_count,
            fat_size_in_sectors,
            fat_type,
            root_dir_sectors,
        })
    }

    pub fn cluster_to_sector(&self, cluster: u32) -> Result<u32, Error> {
        if cluster >= self.cluster_count() {
            return Err(Error::InvalidClusterNumber);
        }

        match self.fat_type {
            FatType::Fat32 => {
                if cluster < self.root_cluster {
                    Err(Error::InvalidClusterNumber)
                } else {
                    Ok((cluster - self.root_cluster) * self.sectors_in_cluster + self.first_data_sector)
                }
            },
            FatType::Fat16 | FatType::Fat12 => {
                if cluster == 0 {
                    Ok(self.first_data_sector - self.root_dir_sectors)
                } else {
                    Ok((cluster - 2) * self.sectors_in_cluster + self.first_data_sector)
                }
            },
        }
    }
}

impl <D: BlockDevice> Fs<D> 
//    where Error: From<<D as BlockDevice>::Error>
{
    pub fn format(dev: &D) -> Result<(), D::Error> {
        println!("Formating...");
        println!("Sectors count: {}", dev.count()?);
        println!("Sectors size: {}", dev.lba_size()?);
        Ok(())
    }
}

fn determine_fat_type_by_clusters_count(count: u32) -> FatType {
    if count < 4085 {
        FatType::Fat12
    } else if count < 65525 {
        FatType::Fat16
    } else {
        FatType::Fat32
    }
}

//const DISK_TABLE_FAT16: [(usize, usize); 8] = [
//    (8400, 0), /* disks up to 4.1 MB, the 0 value for SecPerClusVal trips an error */
//    (32680, 2), /* disks up to 16 MB, 1k cluster */
//    (262144, 4), /* disks up to 128 MB, 2k cluster */
//    (524288, 8), /* disks up to 256 MB, 4k cluster */
//    (1048576, 16), /* disks up to 512 MB, 8k cluster */
//    /* The entries after this point are not used unless FAT16 is forced */
//    (2097152, 32), /* disks up to 1 GB, 16k cluster */
//    (4194304, 64), /* disks up to 2 GB, 32k cluster */
//    (0xFFFFFFFF, 0) /* any disk greater than 2GB, 0 value for SecPerClusVal trips an error */
//];
//
//const DISK_TABLE_FAT32: [(usize, usize); 6] = [
//    (66600, 0), /* disks up to 32.5 MB, the 0 value for SecPerClusVal trips an error */
//    (532480, 1), /* disks up to 260 MB, .5k cluster */
//    (16777216, 8), /* disks up to 8 GB, 4k cluster */
//    (33554432, 16), /* disks up to 16 GB, 8k cluster */
//    (67108864, 32), /* disks up to 32 GB, 16k cluster */
//    (0xFFFFFFFF, 64)/* disks greater than 32GB, 32k cluster */
//];