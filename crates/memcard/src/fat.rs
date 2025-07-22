use crate::dir_entry::DirEntry;
use byteorder::{ReadBytesExt, LE};
use std::cmp::min;
use std::io::{Cursor, Read, Seek};

#[derive(Debug)]
pub struct Superblock {
    pub magic: [u8; 28],
    pub version: [u8; 12],
    pub page_size: u16,
    pub pages_per_cluster: u16,
    pub pages_per_block: u16,
    pub clusters_per_card: u32,
    pub alloc_offset: u32,
    pub alloc_end: u32,
    pub rootdir_cluster: u32,
    pub backup_block1: u32,
    pub backup_block2: u32,
    pub ifc_list: [u32; 32],
    pub bad_block_list: [u32; 32],
    pub card_type: u8,
    pub card_flags: u8,
}

fn read_superblock(c: &mut Cursor<Vec<u8>>) -> std::io::Result<Superblock> {
    let mut magic = [0u8; 28];
    c.read_exact(&mut magic)?;
    let mut version = [0u8; 12];
    c.read_exact(&mut version)?;

    let page_len = c.read_u16::<LE>()?;
    let pages_per_cluster = c.read_u16::<LE>()?;
    let pages_per_block = c.read_u16::<LE>()?;
    let _ = c.read_u16::<LE>()?; // 0xFF00
    let clusters_per_card = c.read_u32::<LE>()?;
    let alloc_offset = c.read_u32::<LE>()?;
    let alloc_end = c.read_u32::<LE>()?;
    let rootdir_cluster = c.read_u32::<LE>()?;
    let backup_block1 = c.read_u32::<LE>()?;
    let backup_block2 = c.read_u32::<LE>()?;
    c.seek_relative(8)?;

    // Indirect FAT Cluster List
    let mut ifc_list = [0u32; 32];
    c.read_u32_into::<LE>(&mut ifc_list)?;
    let mut bad_block_list = [0u32; 32];
    c.read_u32_into::<LE>(&mut bad_block_list)?;

    let card_type = c.read_u8()?;
    let card_flags = c.read_u8()?;

    Ok(Superblock {
        magic,
        version,
        page_size: page_len,
        pages_per_cluster,
        pages_per_block,
        clusters_per_card,
        alloc_offset,
        alloc_end,
        rootdir_cluster,
        backup_block1,
        backup_block2,
        ifc_list,
        bad_block_list,
        card_type,
        card_flags,
    })
}

pub struct Memcard {
    c: Cursor<Vec<u8>>,
    page_size: usize,
    pages_per_cluster: usize,
    ifc_list: [u32; 32],
    pub(crate) rootdir_cluster: usize,
    alloc_offset: usize,
    spare_size: usize,
    raw_page_size: usize,
    cluster_size: usize,
    fat_per_cluster: usize,
    fat_matrix: Vec<Vec<u32>>,
    root_entry: Option<DirEntry>,
    entries_in_root: Vec<DirEntry>,
}

impl Memcard {
    pub fn new(file: Vec<u8>) -> Memcard {
        let mut c = Cursor::new(file);
        let sb = read_superblock(&mut c).unwrap();

        let page_size = sb.page_size as usize;
        let pages_per_cluster = sb.pages_per_cluster as usize;
        let ifc_list: [u32; 32] = sb.ifc_list;
        let rootdir_cluster = sb.rootdir_cluster as usize;
        let alloc_offset = sb.alloc_offset as usize;
        let spare_size = (page_size / 128) * 4;
        let raw_page_size = page_size + spare_size;
        let cluster_size = page_size * pages_per_cluster;
        let fat_per_cluster = cluster_size / 4;

        let mut mc = Memcard {
            c,
            page_size,
            pages_per_cluster,
            ifc_list,
            rootdir_cluster,
            alloc_offset,
            spare_size,
            raw_page_size,
            cluster_size,
            fat_per_cluster,
            fat_matrix: vec![],
            root_entry: None,
            entries_in_root: vec![],
        };

        mc.build_fat_matrix();

        mc
    }

    fn build_matrix(&mut self, cluster_list: Vec<u32>) -> Vec<Vec<u32>> {
        let mut matrix = vec![vec![0; self.fat_per_cluster]; cluster_list.len()];

        for (i, cluster) in cluster_list.iter().enumerate() {
            let mut cluster_value = Cursor::new(self.read_cluster(*cluster));

            cluster_value
                .read_u32_into::<LE>(&mut matrix[i])
                .expect("Failed to read cluster");
        }

        matrix
    }
    fn build_fat_matrix(&mut self) {
        let indirect_fat_matrix = self.build_matrix(self.ifc_list.to_vec());
        let indirect_fat_matrix = Self::flatten_matrix(indirect_fat_matrix);

        let indirect_fat_matrix = indirect_fat_matrix
            .iter()
            .filter(|f| **f != 0xFFFFFFFF)
            .cloned()
            .collect();

        self.fat_matrix = self.build_matrix(indirect_fat_matrix);
    }

    fn flatten_matrix(matrix: Vec<Vec<u32>>) -> Vec<u32> {
        matrix.iter().flatten().cloned().collect()
    }

    fn read_cluster(&mut self, n: u32) -> Vec<u8> {
        let page_index = n as usize * self.pages_per_cluster;
        let mut buffer = vec![];
        for i in 0..self.pages_per_cluster {
            buffer.extend(self.read_page((page_index + i) as u32));
        }

        buffer
    }

    fn read_page(&mut self, n: u32) -> Vec<u8> {
        let offset = self.raw_page_size * n as usize;
        self.c.set_position(offset as u64);
        let mut buffer = vec![0u8; self.page_size];
        self.c.read(&mut buffer).unwrap();

        buffer
    }

    pub fn read_entry_cluster(&mut self, cluster_offset: u32) -> Vec<DirEntry> {
        let buffer = self.read_cluster((cluster_offset as usize + self.alloc_offset) as u32);

        let entry_count = buffer.len() / 512;
        let mut entries = vec![];

        for i in 0..entry_count {
            entries.push(
                DirEntry::from_bytes(&buffer[i * 512..(i + 1) * 512])
                    .expect("Failed to read entry"),
            );
        }

        entries
    }

    pub fn read_data_cluster(&mut self, entry: &DirEntry) -> Vec<u8> {
        let mut buffer = vec![];
        let mut chain_start = entry.cluster;
        let mut bytes_read = 0;

        while chain_start != 0x7FFFFFFF {
            let to_read = min(entry.length as usize - bytes_read, self.cluster_size);
            buffer.extend_from_slice(
                &self.read_cluster(chain_start + self.alloc_offset as u32)[..to_read],
            );
            bytes_read += to_read;
            chain_start = self.get_fat_value(chain_start);
        }

        buffer
    }

    pub fn find_sub_entries(&mut self, parent_entry: &DirEntry) -> Vec<DirEntry> {
        let mut chain_start = parent_entry.cluster;
        let mut sub_entries = vec![];

        while chain_start != 0x7FFFFFFF {
            let entries = self.read_entry_cluster(chain_start as u32);
            for e in entries {
                if sub_entries.len() < parent_entry.length as usize {
                    sub_entries.push(e);
                }
            }
            chain_start = self.get_fat_value(chain_start);
        }

        sub_entries
    }

    fn get_fat_value(&self, n: u32) -> u32 {
        let value = self.fat_matrix[(n as usize / self.fat_per_cluster) % self.fat_per_cluster]
            [n as usize % self.fat_per_cluster];

        if value & 0x80000000 > 0 {
            value ^ 0x80000000
        } else {
            value
        }
    }
}
