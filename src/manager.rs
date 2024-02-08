use crate::elf_reader::*;

enum CurReader {
    MainReader,
    ProgsReader(usize),
}

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


struct manager {
    show_context: bool,
    main_reader: ElfReader,
    cur_reader: CurReader,
    prog_readers: Option<Vec<ElfReader>>,
    trace_log: Vec<FuncInstance>,
    func_stack: Vec<FuncInstance>,
}

impl manager {
    fn new(show_context: bool, main_path: &str, progs_path: Option<Vec<&str>>) -> Self {
        let main_reader = ElfReader::new(0, main_path);
        let prog_readers = if let Some(x) = progs_path {
            let mut prog_readers: Vec<ElfReader> = Vec::new();
            for (idx, i) in x.into_iter().enumerate() {
                prog_readers.push(ElfReader::new(idx as u32, i));
            }
            Some(prog_readers)
        } else {
            None
        };
        
        manager {
            show_context,
            main_reader,
            cur_reader: CurReader::MainReader,
            prog_readers,
            trace_log: Vec::new(),
            func_stack: Vec::new(),
        }
    }

    fn cur_reader(&self) -> &ElfReader {
        match self.cur_reader {
            CurReader::MainReader => &self.main_reader,
            CurReader::ProgsReader(x) => {
                let res = &self.prog_readers
                .as_ref()
                .expect("Option<Vec> is None, should not reach ProgsReader arm")[x]; 
                if res.id as usize == x {
                    res
                } else {
                    panic!("Reader id is not compatable with its index in the vec")
                }
            }
        }
    }

    
}