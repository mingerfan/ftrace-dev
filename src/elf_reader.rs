use elf::{abi::STT_FUNC, endian::AnyEndian, ElfStream};
use std::{cmp::Ordering, fs::File, path::PathBuf};

use crate::{debug_println, debug_print};

#[derive(PartialEq, Eq, Debug)]
#[allow(dead_code)]
enum FunType {
    LocalFunc,
    ExternalFunc,
}

#[allow(dead_code)]
struct Func {
    id: u32,
    func_type: FunType,
    name: String,
    start: u64,
    end: u64,
}

#[allow(dead_code)]
struct FuncInstance {
    id: i32,
    para_num: Vec<u64>,
    start_time: u64,
    end_time: u64,
}

#[allow(dead_code)]
struct ElfReader {
    id: u32,
    file: PathBuf,
    name: String,
    start: u64,
    end: u64,
    func_vec: Vec<Func>,
    call_tracer: Vec<FuncInstance>,
}

impl ElfReader {
    pub fn new(id: u32, file: &PathBuf) -> Self {
        let file_path = file.to_str().expect("Empty file path");
        debug_println!("Elf file: {}", file_path);
        let name = file.file_stem().and_then(|f| f.to_str()).expect("Can not Convert to Str");
        let io = File::open(file).expect("Can not open file");
        let mut file_stream = ElfStream::<AnyEndian, _>::open_stream(io).expect("Open Failed");

        let text_shdr = *file_stream.section_header_by_name(".text")
        .expect("Section table should be parseable")
        .expect("File should have a .text section");
        // 不确定.text文件中是否就是代码，它的起始和结束是否就是在代码的起始的结束
        let start = text_shdr.sh_addr;
        let end = start + text_shdr.sh_offset;

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

            Func { id: idx as u32, func_type, name: func_name.to_string(), start: func_start, end: func_end }
        }).collect::<Vec<Func>>();

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
            debug_println!("Get function: {}, id: {}, start: {}, end: {}, type: {:?}", x.name, x.id, x.start, x.end, x.func_type);
        });

        ElfReader {
            id,
            file: file.to_owned(),
            name: name.to_string(),
            start,
            end,
            func_vec,
            call_tracer: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_new() {
        let file = PathBuf::from("./test_elf/riscv64-nemu-interpreter");
        let elf_reader0 = ElfReader::new(0, &file);
        println!("ElfReader(id: {}, file: {}, start: {}, end: {}, func_num: {})", 
        elf_reader0.id, elf_reader0.name, elf_reader0.start, elf_reader0.end, elf_reader0.func_vec.len());
        for i in 0..elf_reader0.func_vec.len() {
            assert!(elf_reader0.func_vec[i].id == i as u32);
        }
    }
}