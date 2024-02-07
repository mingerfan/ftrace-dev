use crate::elf_reader::*;


struct FuncInstance {
    id: u32,
    func_type: FunType,
    ret_val: Option<(u64, Option<u64>)>,
    paras: Option<Vec<u64>>,
    start_time: u64,
    end_time: u64,
}

impl FuncInstance {
    fn new(id: u32, func_type: FunType, start_time: u64, paras: Option<Vec<u64>>) -> Self {
        FuncInstance {
            id,
            func_type,
            ret_val: None,
            paras,
            start_time,
            end_time: start_time,
        }
    }
}


struct manager<'a> {
    show_context: bool,
    main_reader: ElfReader,
    cur_reader: &'a ElfReader,
    prog_readers: Option<Vec<ElfReader>>,
    trace_log: Vec<FuncInstance>,
    func_stack: Vec<FuncInstance>,
}

// impl manager {
//     fn new(show_context: bool, main_path: &str, progs_path: Option<Vec<&str>>) {
//         let main_reader = ElfReader::new(0, );

//         let progs_reader = Vec::new();
        
//     }
// }