use byteorder::{ReadBytesExt, LE};
use std::cmp::min;
use std::io;
use std::io::{Cursor, Read, Seek};
use crate::dir_entry::{DirEntry, DF_EXISTS};

#[derive(Debug)]
struct FATEntry {
    next_cluster: u32,
    occupied: bool,
    raw: u32,
}

impl FATEntry {
    fn from(value: u32) -> Self {
        Self {
            next_cluster: value & 0x7FFFFFFF,
            occupied: value & 0x80000000 > 0,
            raw: value,
        }
    }
}

pub struct Memcard {
    c: Cursor<Vec<u8>>,
    pub superblock: Superblock,
    ecc_bytes: u32,
    page_spare_area_size: usize,
}

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

impl Memcard {
    pub(crate) fn new(data: Vec<u8>) -> Self {
        let size = data.len();
        let mut c = Cursor::new(data);
        let superblock = read_superblock(&mut c).unwrap();

        let expected_size = superblock.clusters_per_card
            * superblock.pages_per_cluster as u32
            * superblock.page_size as u32;
        let expected_size_with_ecc = (superblock.clusters_per_card
            * superblock.pages_per_cluster as u32
            * (superblock.page_size as u32 + 16)) as usize;

        let mut ecc_bytes = 0;
        let mut page_spare_area_size = 0;

        if size == expected_size_with_ecc {
            ecc_bytes = 12;
            page_spare_area_size = 16;
        }

        Self {
            c,
            superblock,

            ecc_bytes,
            page_spare_area_size,
        }
    }
    fn page_size(&self) -> usize {
        self.superblock.page_size as usize + self.page_spare_area_size
    }

    fn page_capacity(&self) -> usize {
        self.superblock.page_size as usize
    }

    fn cluster_size(&self) -> usize {
        self.page_size() * self.superblock.pages_per_cluster as usize
    }

    fn cluster_capacity(&self) -> usize {
        self.page_capacity() * self.superblock.pages_per_cluster as usize
    }

    fn logical_to_physical_offset(&mut self, cluster: u32, offset: usize) -> io::Result<usize> {
        let k_capacity = self.cluster_capacity();
        let k_size = self.cluster_size();
        let p_capacity = self.page_capacity();
        let p_size = self.page_size();

        let mut cluster = self.seek(cluster, offset / k_capacity)?;
        assert_ne!(cluster, 0xFFFFFFFF);

        let offset = offset % k_capacity;
        cluster += self.superblock.alloc_offset;

        Ok(cluster as usize * k_size + offset / p_capacity * p_size)
    }

    fn get_entry_offest(&mut self, cluster: u32) -> io::Result<u32> {
        let cluster_capacity = self.cluster_capacity() as u32;
        let cluster_size = self.cluster_size() as u32;

        let k = cluster_capacity / 4; // 4 = u32
        let fat_offset = cluster % k;
        let indirect_index = cluster / k;
        let indirect_offset = indirect_index % k;
        let dbl_indirect_index = indirect_index / k;
        let indirect_cluster_num = self.superblock.ifc_list[dbl_indirect_index as usize];
        // println!("[get_entry_offset] cluster: {}, k: {}, indirect_index: {}, dbl: {}, indirect_cluster: {:#X}", cluster, k, indirect_index, dbl_indirect_index, indirect_cluster_num);

        let fat_cluster_offest = indirect_cluster_num * cluster_size + indirect_offset * 4;
        self.c.set_position(fat_cluster_offest as u64);
        let fat_cluster_num = self.c.read_u32::<LE>()?;
        // println!("[get_entry_offset] fat_cluster_num: {:#X}", fat_cluster_num);

        Ok(fat_cluster_num * cluster_size + fat_offset * 4)
    }

    fn get_table_entry(&mut self, cluster: u32) -> io::Result<FATEntry> {
        let offset = self.get_entry_offest(cluster)? as u64;
        self.c.set_position(offset);
        let val = self.c.read_u32::<LE>()?;
        let entry = FATEntry::from(val);
        // println!("[get_table_entry] cluster: {:#X}, offset: {}, val: {:#X}, occupied: {}", cluster, offset, val, entry.occupied);
        Ok(entry)
    }

    fn seek(&mut self, cluster: u32, count: usize) -> io::Result<u32> {
        let mut cluster = cluster;
        // println!("[seek] start cluster = {:#X}, count = {}", cluster, count);

        for i in 0..count {
            let fat_value = self.get_table_entry(cluster)?;
            // println!("[seek] hop {}: cluster = {:#X}, next = {:#X}, occupied = {}", i, cluster, fat_value.next_cluster, fat_value.occupied);

            if fat_value.raw == 0xFFFFFFFF || !fat_value.occupied {
                // println!("[seek] invalid entry at cluster = {:#X}", cluster);
                return Ok(0xFFFFFFFF);
            }
            cluster = fat_value.next_cluster;
        }

        // println!("[seek] resolved to cluster = {:#X}", cluster);
        Ok(cluster)
    }

    fn read_fat(&mut self, cluster: u32, offset: usize, buf_size: usize) -> io::Result<Vec<u8>> {
        if cluster == 0xFFFFFFFF {
            return Ok(vec![]);
        }

        let mut cluster = cluster;
        let mut offset = offset;
        let mut buf_offset = 0;

        let k_capacity = self.cluster_capacity();
        let p_capacity = self.page_capacity();
        let p_size = self.page_size();

        let mut read_buf = vec![0; buf_size];
        let mut page_buffer = vec![0u8; p_size];

        while buf_offset < buf_size {
            if cluster == 0xFFFFFFFF {
                break;
            }

            cluster = self.seek(cluster, offset / k_capacity)?;
            offset %= k_capacity;

            let mc_offset = self.logical_to_physical_offset(cluster, offset)?;

            let buffer_left = buf_size - buf_offset;
            let page_left = p_capacity - offset % p_capacity;
            let s = min(buffer_left, page_left);

            // let spare_start = mc_offset + p_capacity;

            self.c.set_position(mc_offset as u64);
            self.c.read_exact(&mut page_buffer)?;

            // println!("Filled {} / {}", buf_offset, buf_size);

            read_buf[buf_offset..buf_offset + s].copy_from_slice(&page_buffer[..s]);

            buf_offset += s;
            offset += s;
        }

        Ok(read_buf)
    }

    pub fn get_child(&mut self, cluster: u32, count: usize) -> io::Result<DirEntry> {
        Ok(DirEntry::from_bytes(&self.read_fat(cluster, count * 512, 512)?)?)
    }

    pub fn ls(&mut self, parent: &DirEntry) -> io::Result<Vec<DirEntry>> {
        let dirents_per_cluster = self.cluster_capacity() / 512; // DIR_ENTRY_SIZE
        let mut dirents = Vec::new();

        let mut cluster = parent.cluster;

        for i in 0.. parent.length as usize {
            if i % dirents_per_cluster == 0 && i != 0 {
                cluster = self.seek(cluster, 1)?;
                if cluster == 0xFFFFFFFF {
                    break
                }
            }

            let child = self.get_child(cluster, i % dirents_per_cluster)?;

            if child.mode & DF_EXISTS > 0 {
                dirents.push(child);
            } else {
                // Skip deleted files
                // eprintln!("Skipping deleted")
            }
        }

        Ok(dirents)
    }

    pub fn read(&mut self, dir_entry: &DirEntry, size: usize, offset: usize) -> io::Result<Vec<u8>> {
        let mut size = size;

        if offset >= dir_entry.length as usize {
            return Ok(vec![]);
        }
        if offset + size > dir_entry.length as usize {
            size = dir_entry.length as usize - offset;
        }

        self.read_fat(dir_entry.cluster, offset, size)
    }
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
