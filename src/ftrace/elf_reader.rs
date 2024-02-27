use elf::{abi::STT_FUNC, endian::AnyEndian, ElfStream};
use std::{cmp::Ordering, fs::File, path::PathBuf};

use crate::{debug_println, debug_print};

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
#[non_exhaustive]
pub enum FunType {
    LocalFunc,
    ExternalFunc,
}

#[derive(Clone)]
pub struct Func {
    pub id: u32,
    pub func_type: FunType,
    pub name: String,
    pub start: u64,
    pub end: u64,
}


#[derive(Clone)]
pub struct ElfReader {
    pub id: u32,
    pub name: String,
    pub start: u64,
    pub end: u64,
    func_vec: Vec<Func>,
}

impl ElfReader {
    pub fn new(id: u32, file: &str) -> Self {
        debug_println!("Elf file: {}", file);
        let file = &PathBuf::from(file);
        let name = file.file_stem().and_then(|f| f.to_str()).expect("Can not Convert to Str");
        let io = File::open(file).expect("Can not open file");
        let mut file_stream = ElfStream::<AnyEndian, _>::open_stream(io).expect("Open Failed");

        // let text_shdr = *file_stream.section_header_by_name(".text")
        // .expect("Section table should be parseable")
        // .expect("File should have a .text section");
        // 不确定.text文件中是否就是代码，它的起始和结束是否就是在代码的起始的结束
        // let start = text_shdr.sh_addr;
        // let end = start + text_shdr.sh_offset;

        let (sym_t, str_t) = file_stream.symbol_table()
        .expect("Section table should be parseable")
        .expect("File should have a .Symtab section");
        let mut func_vec = sym_t.iter().filter(|x| x.st_symtype() == STT_FUNC)
        .enumerate()
        .map(|(idx, x)| {
            let func_name = str_t.get(x.st_name as usize).expect("Invalid index of StringTable");
            let func_start = x.st_value;
            let func_end = x.st_size + func_start;

            let func_type = if func_start == func_end && func_start == 0 { FunType::ExternalFunc } 
            else { FunType::LocalFunc };
            // 似乎end是开区间
            Func { id: idx as u32, func_type, name: func_name.to_string(), start: func_start, end: func_end }
        }).filter(|x| x.func_type == FunType::LocalFunc).collect::<Vec<Func>>();

        func_vec.sort_by(|a, b| {
            if a.func_type == FunType::ExternalFunc {
                Ordering::Greater
            } else if b.func_type == FunType::ExternalFunc {
                Ordering::Less
            } else {
                a.start.cmp(&b.start)
            }
        });

        func_vec.iter_mut().enumerate().for_each(|(i, f)| {
            f.id = i as u32;
        });

        func_vec.iter().for_each(|x| {
            if x.start == x.end && x.func_type == FunType::LocalFunc {
                debug_print!("----- ");
            }
            debug_println!("Get function: {}, id: {}, start: 0x{:X}, end: 0x{:X}, type: {:?}", x.name, x.id, x.start, x.end, x.func_type);
        });

        let start = func_vec.first().expect("Failed to get the first elements in func_vec").start;
        let end = func_vec.last().expect("Failed to get the last elements in func_vec").end;

        ElfReader {
            id,
            name: name.to_string(),
            start,
            end,
            func_vec,
        }
    }

    #[cfg(test)]
    pub fn dummy(id: u32, name: &str, start: u64, end: u64, func_vec: Option<Vec<Func>>) -> ElfReader {
        ElfReader {
            id,
            name: name.to_string(),
            start,
            end,
            func_vec: if let Some(func_vec) = func_vec {
                func_vec
            } else {
                Vec::new()
            }
        }
    }

    pub fn find(&self, value: u64) -> Option<&Func> {
        self.func_vec.binary_search_by(|x| {
            if value < x.start {
                Ordering::Greater
            } else if value >= x.end {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }).ok().and_then(|idx| self.func_vec.get(idx))
    }

    pub fn get_func(&self, id: u32) -> Option<&Func>{
        self.func_vec.get(id as usize)
        .and_then(|x| {
            if x.id == id {
                Some(x)
            } else {
                debug_println!("Func id is not compatible with Vec index");
                None
            }
        })
    }
    #[allow(dead_code)]
    pub fn func_vec(&self) -> &[Func] {
        &self.func_vec
    }

    pub fn reader_cmp(&self, pc: u64) -> Ordering {
        if self.start > pc {
            Ordering::Greater
        } else if self.end < pc {
            // 虽然end的确是开区间
            // 但是需要考虑到一些elf的结尾函数是start==end的情况
            // 所以故意这么写
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn create_new(id: u32, path: &str) -> ElfReader {
        ElfReader::new(id, path)
    }

    #[test]
    // 这是x86的elf
    fn test_reader_new() {
        let elf_reader0 = create_new(0, "./test_elf/riscv64-nemu-interpreter");
        println!("ElfReader(id: {}, name: {}, start: 0x{:X}, end: 0x{:X}, func_num: {})", 
        elf_reader0.id, elf_reader0.name, elf_reader0.start, elf_reader0.end, elf_reader0.func_vec.len());
        for i in 0..elf_reader0.func_vec.len() {
            assert!(elf_reader0.func_vec[i].id == i as u32);
        }
    }

    #[test]
    // 这是riscv的elf
    fn test_reader_new1() {
        let elf_reader1 = create_new(1, "./test_elf/nanos-lite-riscv64-nemu.elf");
        println!("ElfReader(id: {}, name: {}, start: 0x{:X}, end: 0x{:X}, func_num: {})", 
        elf_reader1.id, elf_reader1.name, elf_reader1.start, elf_reader1.end, elf_reader1.func_vec.len());
        for i in 0..elf_reader1.func_vec.len() {
            assert!(elf_reader1.func_vec[i].id == i as u32);
        }
    }

    #[test]
    fn test_find() {
        let elf_reader = create_new(0, "./test_elf/riscv64-nemu-interpreter");
        let mut rng = rand::thread_rng();
    
        println!("\n===============To Test gap miss================");
        let mut last_end = elf_reader.func_vec.first().unwrap().end;
        for i in &elf_reader.func_vec {
            if i.start > last_end {
                let rand_addr = rng.gen_range(last_end..i.start);
                print!("Random addr 0x{:X} between 0x{:X} and 0x{:X}, should miss\t", rand_addr, i.start, last_end);
                assert!(elf_reader.find(rand_addr).is_none());
                println!("Miss!")
            }
            last_end = i.end;
        }
    }

    #[test]
    // riscv elf
    fn test_find1() {
        let elf_reader1 = create_new(1, "./test_elf/nanos-lite-riscv64-nemu.elf");
        let mut rng = rand::thread_rng();
        println!("ElfReader(id: {}, name: {}, start: 0x{:X}, end: 0x{:X}, func_num: {})", 
        elf_reader1.id, elf_reader1.name, elf_reader1.start, elf_reader1.end, elf_reader1.func_vec.len());

        println!("\n===============To Test hit================");
        for i in 0..elf_reader1.func_vec.len() {
            let start = elf_reader1.func_vec[i].start;
            let end = elf_reader1.func_vec[i].end;
            if start == end { continue; }
            let rand_addr = rng.gen_range(start..end);
            print!("Random addr: 0x{:X}, start: 0x{:X}, end: 0x{:X}\t", rand_addr, start, end);
            assert!(elf_reader1.find(rand_addr).is_some());
            println!("Hit!");
        }

        println!("\n===============To Test miss================");
        let addr = elf_reader1.func_vec.first().unwrap().start - 1;
        print!("Should miss addr(start - 1): 0x{:X}\t", addr);
        assert!(elf_reader1.find(addr).is_none());
        println!("Miss!");

        let addr = elf_reader1.func_vec.last().unwrap().end;
        print!("Should miss addr(end): 0x{:X}\t", addr);
        assert!(elf_reader1.find(addr).is_none());
        println!("Miss!");

        let addr = addr + 1;
        print!("Should miss addr(end + 1): 0x{:X}\t", addr);
        assert!(elf_reader1.find(addr).is_none());
        println!("Miss!");
    }

    #[test]
    fn test_get_func() {
        let elf_reader = create_new(0, "./test_elf/riscv64-nemu-interpreter");
        for i in 0..elf_reader.func_vec.len() {
            let func = elf_reader.get_func(i as u32).unwrap();
            println!("Func: {}, id: {}", func.name, func.id);
        }
        assert!(elf_reader.get_func(elf_reader.func_vec.len() as u32).is_none());
    }

}