use crate::elf_reader::*;
use crate::debug_println;
// use std::cmp::Ordering;

#[derive(PartialEq, Eq)]
enum CurReader {
    MainReader,
    ProgsReader(usize),
}

struct FuncInstance {
    // 这里instance的id主要是用于结合cur_reader定位函数信息位置的
    id: u32,
    reader: CurReader,
    func_type: FunType,
    ret_val: Option<(u64, Option<u64>)>,
    paras: Option<Vec<u64>>,
    start_time: u64,
    end_time: u64,
}

impl FuncInstance {
    fn new(id: u32, func_type: FunType, reader: CurReader, start_time: u64, paras: Option<Vec<u64>>) -> Self {
        FuncInstance {
            id,
            reader,
            func_type,
            ret_val: None,
            paras,
            start_time,
            end_time: start_time,
        }
    }

    fn set_end_time(&mut self, end_time: u64) {
        self.end_time = end_time
    }

    fn set_ret_val(&mut self, ret_val: (u64, Option<u64>), show_context: bool) {
        if show_context {
            self.ret_val = Some(ret_val);
        } else {
            self.ret_val = None;
        }
    }

    fn set_end_and_ret(&mut self, end_time: u64, ret_val: (u64, Option<u64>), show_context: bool) {
        self.set_end_time(end_time);
        self.set_ret_val(ret_val, show_context);
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
                prog_readers.push(ElfReader::new((idx+1) as u32, i));
            }
            prog_readers.sort_by(|a, b| a.start.cmp(&b.start));
            for i in &prog_readers {
                debug_println!("Progs elf reader: name {}, id {}", i.name, i.id);
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