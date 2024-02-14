use crate::elf_reader::*;
use crate::debug_println;
// use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use std::rc::Rc;

#[derive(PartialEq, Eq, Clone, Copy)]
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


pub struct Manager {
    show_context: bool,
    main_reader: ElfReader,
    cur_reader: CurReader,
    prog_readers: Option<Vec<ElfReader>>,
    trace_log: Vec<Rc<FuncInstance>>,
    func_stack: Vec<Rc<FuncInstance>>,
    init_time: u64,
}

impl Manager {
    // 这里依靠reader保证start一定小于等于end
    // 同时这里一定是有序(start有序）的序列
    // 如果有重叠返回true，否则返回false
    fn check_reader_overlap(main_readers: &ElfReader, readers: Option<Vec<&ElfReader>>) -> bool {
        if readers.is_none() {
            false
        } else if let Some(x) = readers {
            let mut res = false;
            for (i, &item) in x.iter().enumerate() {
                assert!(item.start <= item.end);
                let next = x.get(i+1);
                if let Some(&next) = next {
                    res = res || (item.end > next.start);
                } 
            };

            for i in x {
                res = res || (main_readers.start >= i.start && main_readers.start < i.end);
            }
            res
        } else {
            panic!("Should not reach here!")
        }
    }

    pub fn new(show_context: bool, main_path: &str, progs_path: Option<Vec<&str>>) -> Self {
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
    
        let init_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
        .as_millis()
        as u64;
        
        Manager {
            show_context,
            main_reader,
            cur_reader: CurReader::MainReader,
            prog_readers,
            trace_log: Vec::new(),
            func_stack: Vec::new(),
            init_time,
        }
    }

    fn get_time(&self) -> u64 {
        let time = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        as u64;
        time - self.init_time
    }

    fn get_reader(&self, reader: &CurReader) -> &ElfReader {
        match *reader {
            CurReader::MainReader => &self.main_reader,
            CurReader::ProgsReader(x) => {
                let res = &self.prog_readers
                .as_ref()
                .expect("Option<Vec> is None, should not reach ProgsReader arm")[x];
                if res.id as usize == x {
                    res
                } else {
                    panic!("Reader id is not compatible with its index in the vec")
                }
            }
        }
    }

    fn cur_reader(&self) -> &ElfReader {
        self.get_reader(&self.cur_reader)
    }

    fn func_reader(&self, func: &FuncInstance) -> &ElfReader {
        self.get_reader(&func.reader)
    }


    pub fn first_add_function(&mut self, pc: u64, paras: Option<Vec<u64>>) {
        assert!(self.cur_reader == CurReader::MainReader, "Is not first function");
        assert!(self.func_stack.is_empty(), "Is not first function");
        assert!(self.trace_log.is_empty(), "Is not first function");
        let func_info = match self.cur_reader().find(pc) {
            Some(x) => {
                debug_println!("Init function add {} in Main Reader", x.name);
                FuncInstance::new(x.id, FunType::LocalFunc, 
                    CurReader::MainReader, 
                    self.get_time(), paras)
            },
            None =>{
                debug_println!("Init function add anonymous function");
                // 此时没有检测到call，就应该认为是一个匿名的头部被添加
                FuncInstance::new(0, FunType::ExternalFunc, 
                    CurReader::MainReader, 
                    self.get_time(), paras)
            }
        };
        let func_info = Rc::new(func_info);
        self.trace_log.push(func_info.clone());
        self.func_stack.push(func_info);
    }
    

}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    fn create_new(id: u32, path: &str) -> ElfReader {
        ElfReader::new(id, path)
    }

    #[test]
    fn test_check_overlap() {
        let reader = create_new(0, "./test_elf/riscv64-nemu-interpreter");
        let reader1 = create_new(1, "./test_elf/nanos-lite-riscv64-nemu.elf");
        let dummy = ElfReader::dummy(0, "dummy", 0x50000, 0x50005, None);
        let dummy1 = ElfReader::dummy(1, "dummy1", 0x50005, 0x50008, None);

        println!("============To test check overlap============");
        print!("Test the same reader, should true:\t");
        let res = Manager::check_reader_overlap(&reader, Some(vec![&reader]));
        assert!(res);
        println!("True!");

        print!("Test the same reader(2), should true:\t");
        let res = Manager::check_reader_overlap(&reader1, Some(vec![&reader1]));
        assert!(res);
        println!("True!");

        print!("Only main reader, should false:\t");
        let res = Manager::check_reader_overlap(&reader, None);
        assert!(!res);
        println!("False!");

        print!("Test different reader, should false:\t");
        let mut vec = vec![&reader1, &dummy, &dummy1];
        vec.sort_by(|a, b| a.start.cmp(&b.start));
        let res = Manager::check_reader_overlap(&reader, Some(vec));
        assert!(!res);
        println!("False!");

        print!("Test different reader(2), should false:\t");
        let mut vec = vec![&reader, &reader1, &dummy1];
        vec.sort_by(|a, b| a.start.cmp(&b.start));
        let res = Manager::check_reader_overlap(&dummy, Some(vec));
        assert!(!res);
        println!("False!");
    }
}