use crate::elf_reader::*;

#[allow(dead_code)]
struct FuncInstance {
    id: u32,
    func_type: FunType,
    para_num: Vec<u64>,
    start_time: u64,
    end_time: u64,
}

