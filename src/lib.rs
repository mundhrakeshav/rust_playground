#[macro_use]
extern crate serde_derive;

extern crate byteorder;
extern crate crc;
use std::io::prelude::*;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crc::Crc;

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom},
    path::Path,
};

type ByteString = Vec<u8>;
type ByteStr = [u8];

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: ByteString,
    pub value: ByteString,
}

#[derive(Debug)]
pub struct ActionKV {
    f: File,
    pub index: HashMap<ByteString, u64>,
}

const CRC32: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_CKSUM);

impl ActionKV {
    pub fn open(path: &Path) -> io::Result<Self> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .append(true)
            .create(true)
            .open(path)?;

        return Ok(ActionKV {
            f: f,
            index: HashMap::new(),
        });
    }

    pub fn load(&mut self) -> io::Result<()> {
        let mut f = BufReader::new(&mut self.f);

        loop {
            let current_position = f.seek(SeekFrom::Current(0))?;
            println!("{}", current_position);
            let maybe_kv: Result<KeyValuePair, io::Error> = ActionKV::process_record(&mut f);

            let kv: KeyValuePair = match maybe_kv {
                Ok(kv) => kv,
                Err(err) => match err.kind() {
                    io::ErrorKind::UnexpectedEof => {
                        break;
                    }

                    _ => return Err(err),
                },
            };

            self.index.insert(kv.key, current_position);
        }

        Ok(())
    }

    fn process_record<R: Read>(f: &mut R) -> io::Result<KeyValuePair> {
        let saved_checksum = f.read_u32::<LittleEndian>()?;
        let key_len = f.read_u32::<LittleEndian>()?;
        let val_len = f.read_u32::<LittleEndian>()?;
        let data_len = key_len + val_len;
        let mut data = ByteString::with_capacity(data_len as usize);
        f.by_ref().take(data_len as u64).read_to_end(&mut data)?;
        debug_assert_eq!(data.len(), data_len as usize);

        let checksum = CRC32.checksum(&data);
        if checksum != saved_checksum {
            panic!(
                "data corruption encountered ({:08x} != {:08x})",
                checksum, saved_checksum
            );
        }

        let val = data.split_off(key_len as usize);
        let key = data;
        Ok(KeyValuePair {
            key: key,
            value: val,
        })
    }
    pub fn insert(&mut self, key: &ByteStr, value: &ByteStr) -> io::Result<()> {
        let pos = self.insert_but_ignore_index(key, value)?;
        self.index.insert(key.to_vec(), pos);
        Ok(())
    }

    fn insert_but_ignore_index(&mut self, key: &ByteStr, value: &ByteStr) -> io::Result<u64> {
        let mut f = BufWriter::new(&mut self.f);
        let key_len = key.len();
        let val_len = value.len();

        let mut tmp = ByteString::with_capacity(key_len + val_len);

        for byte in key {
            tmp.push(*byte);
        }

        for byte in value {
            tmp.push(*byte);
        }

        let checksum = CRC32.checksum(&tmp);
        let next_byte = SeekFrom::End(0);
        let cur_pos = f.seek(SeekFrom::Current(0))?;
        f.seek(next_byte)?;
        let _ = f.write_u32::<LittleEndian>(key_len as u32);
        let _ = f.write_u32::<LittleEndian>(val_len as u32);
        let _ = f.write_u32::<LittleEndian>(checksum);
        let _ = f.write_all(&tmp)?;
        // Print checksum len
        println!(
            "{:?} {:?} {:?}",
            (key_len as u32).to_le_bytes(),
            (val_len as u32).to_le_bytes(),
            checksum.to_le_bytes(),
        );
        println!(
            "{:?} {:?} {:?}",
            (key_len as u32).to_be_bytes(),
            (val_len as u32).to_be_bytes(),
            checksum.to_be_bytes(),
        );
        println!(
            "{:?} {:?} {:?}",
            (key_len as u32).to_ne_bytes(),
            (val_len as u32).to_ne_bytes(),
            checksum.to_ne_bytes(),
        );
        Ok(cur_pos)
    }

    #[inline]
    pub fn update(&mut self, key: &ByteStr, value: &ByteStr) -> io::Result<()> {
        self.insert(key, value)
    }

    #[inline]
    pub fn delete(&mut self, key: &ByteStr) -> io::Result<()> {
        self.insert(key, b"")
    }

    pub fn get(&mut self, key: &ByteStr) -> io::Result<Option<ByteString>> {
        // <4>
        let position = match self.index.get(key) {
            None => return Ok(None),
            Some(position) => *position,
        };

        let kv = self.get_at(position)?;

        Ok(Some(kv.value))
    }

    pub fn get_at(&mut self, position: u64) -> io::Result<KeyValuePair> {
        let mut f = BufReader::new(&mut self.f);
        f.seek(SeekFrom::Start(position))?;
        let kv = ActionKV::process_record(&mut f)?;

        Ok(kv)
    }
}
