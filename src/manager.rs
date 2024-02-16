use crate::elf_reader::*;
use crate::debug_println;
use std::cmp::Ordering;
// use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use std::rc::Rc;

#[derive(PartialEq, Eq, Clone, Copy)]
enum CurReader {
    MainReader,
    ProgReaders(usize),
}

pub struct FuncInstance {
    // 这里instance的id主要是用于结合cur_reader定位函数信息位置的
    id: u32,
    reader: Option<CurReader>,
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
            reader: Some(reader),
            func_type,
            ret_val: None,
            paras,
            start_time,
            end_time: start_time,
        }
    }

    fn new_with_nullreader(id: u32, start_time: u64, paras: Option<Vec<u64>>) -> Self {
        // 没有reader的函数一定时external的
        FuncInstance {
            id,
            reader: None,
            func_type: FunType::ExternalFunc,
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

    fn ret_val(&self) -> Option<(u64, Option<u64>)> {
        self.ret_val
    }

    fn paras(&self) -> Option<&Vec<u64>> {
        self.paras.as_ref()
    }

    fn start_time(&self) -> u64 {
        self.start_time
    }

    fn end_time(&self) -> u64{
        self.end_time
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

        let prog_readers_ref = prog_readers
        .as_ref()
        .map(|x| x.iter().collect());
        Self::check_reader_overlap(&main_reader, prog_readers_ref);
        
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

    pub fn get_time(&self) -> u64 {
        let time = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        as u64;
        time - self.init_time
    }

    fn get_reader(&self, reader: &CurReader) -> &ElfReader {
        match *reader {
            CurReader::MainReader => &self.main_reader,
            CurReader::ProgReaders(x) => {
                let res = &self.prog_readers
                .as_ref()
                .expect("Option<Vec> is None, should not reach ProgReaders arm")[x];
                if res.id as usize == x {
                    res
                } else {
                    panic!("Reader id is not compatible with its index in the vec")
                }
            }
        }
    }

    pub fn cur_reader(&self) -> &ElfReader {
        self.get_reader(&self.cur_reader)
    }

    pub fn func_reader(&self, func: &FuncInstance) -> Option<&ElfReader> {
        if let Some(reader) = func.reader.as_ref() {
            Some(self.get_reader(reader))
        } else {
            None
        }
    }


    fn first_add_function(&mut self, pc: u64, paras: Option<Vec<u64>>) {
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

    fn check_bound(&self, func_ins: &FuncInstance, pc: u64) -> bool {
        // 确认pc在函数的范围内，如果在范围内返回true，不在就返回false
        let reader = self.func_reader(func_ins);
        if let Some(reader) = reader {
            let func = reader.get_func(func_ins.id)
            .expect("Can not get function from function instance, maybe illegal instance");
            if func.func_type == FunType::LocalFunc {
                // 如果是local func，用func自带的bound进行判断
                (pc >= func.start) && (pc < func.end)
            } else {
                // 如果是external func，可以用func的上下函数进行判断
                // 如果在上下函数之间，我们姑且认为是同一个函数
                let upper_bound = if func.id == 0 {
                    reader.start
                } else {
                    reader.get_func(func.id - 1).expect("Unexpected behaviour")
                    .end
                };
                let lower_bound = reader.get_func(func.id + 1)
                .map(|x| x.start)
                .unwrap_or(reader.end);
                (pc >= upper_bound) && (pc < lower_bound)
            }
        } else {
            // 没有reader一定是外部函数，而且无法判断，进行assert确认，
            // 并且返回false，表示始终在func_ins的范围外
            false
        }
    }

    fn elfreader_to_curreader(&self, reader: &ElfReader) -> CurReader {
        // 这里与new的时候分配给各个reader的id相关, 其中，主id为0，其它从1开始
        // 需要进行校验
        let id = reader.id;
        if id == 0 {
            assert!(id == self.main_reader.id);
            assert!(reader.start == self.main_reader.start);
            assert!(reader.end == self.main_reader.end);
            assert!(reader.name == self.main_reader.name);
            CurReader::MainReader
        } else {
            // 此时不在main reader，需要进行各种校验
            if let Some(x) = &self.prog_readers {
                let res_reader = x.get((id-1) as usize)
                .expect("Can not find target prog reader");
                assert!(id == res_reader.id);
                assert!(reader.start == res_reader.start);
                assert!(reader.end == res_reader.end);
                assert!(reader.name == res_reader.name);
                CurReader::ProgReaders((id-1) as usize)
            } else {
                panic!("Prog readers vec does not exist, convert failed!");
            }
        }
    }

    fn build_ins_and_push(&mut self, cur_reader: CurReader, pc: u64, paras: Option<Vec<u64>>) {
        // 这里假设了已经找到了pc对应的reader
        let reader = self.get_reader(&cur_reader);
        let func = reader.find(pc);
        if let Some(named_func) = func {
            let func_ins = FuncInstance::new(named_func.id, named_func.func_type, 
                cur_reader, 
                self.get_time(), 
                paras);
            let func_ins = Rc::new(func_ins);
            self.trace_log.push(func_ins.clone());
            self.func_stack.push(func_ins);
        } else {
            // 如果没有找到，那就是匿名函数
            let func_ins = FuncInstance::new(0, FunType::ExternalFunc, 
                cur_reader, 
                self.get_time(), 
                paras);
            let func_ins = Rc::new(func_ins);
            self.trace_log.push(func_ins.clone());
            // 由于是匿名函数，所以应该检查栈顶部是否是匿名函数
            // 如果是就不继续添加，不是就继续添加
            if let Some(x) = self.func_stack.last() {
                if x.func_type == FunType::LocalFunc {
                    self.func_stack.push(func_ins);
                }
            }
        }
        
    }
    
    fn noram_add_funtion(&mut self, pc: u64, paras: Option<Vec<u64>>) {
        // 这个函数假设了已经需要切换函数（也就是check_bound失败）
        // 这个函数需要切换cur reader
        assert!(!self.trace_log.is_empty());
        let cur_reader = self.cur_reader();
        if cur_reader.reader_cmp(pc) == Ordering::Equal {
            let reader_enum = self.elfreader_to_curreader(cur_reader);
            self.build_ins_and_push(reader_enum, pc, paras);
        } else if self.main_reader.reader_cmp(pc) == Ordering::Equal {
            let reader_enum = self.elfreader_to_curreader(&self.main_reader);
            self.cur_reader = reader_enum;
            self.build_ins_and_push(reader_enum, pc, paras);
        } else {
            let readers = self.prog_readers.as_ref()
            .expect("Can not find prog readers vec, abort!");
            let reader_opt = readers.iter().find(|x| {
                x.reader_cmp(pc) == Ordering::Equal
            });
            if let Some(reader) = reader_opt {
                let reader_enum = self.elfreader_to_curreader(reader);
                self.cur_reader = reader_enum;
                self.build_ins_and_push(reader_enum, pc, paras);
            } else {
                // 此时就是不在所有elf范围的外部（匿名）函数
                // 这时候打印一些信息，但是仍然作为外部函数进行添加
                debug_println!("The current pc: {} does not have a compatible reader, 
                add as an anonymous function instance", pc);
                let func_ins = FuncInstance::new_with_nullreader(0, self.get_time(), paras);
                let func_ins = Rc::new(func_ins);
                self.trace_log.push(func_ins.clone());
                if let Some(x) = self.func_stack.last() {
                    if x.func_type == FunType::LocalFunc {
                        self.func_stack.push(func_ins);
                    }
                }
            }
        }
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

    #[test]
    fn test_converter() {
        let manager = Manager::new(false, 
            "./test_elf/riscv64-nemu-interpreter", 
            Some(vec!["./test_elf/nanos-lite-riscv64-nemu.elf"]));
        let main_reader = &manager.main_reader;
        let prog_reader = &manager.prog_readers.as_ref().unwrap()[0];
        println!("============To test converter============");
        assert!(manager.elfreader_to_curreader(main_reader) == CurReader::MainReader);
        assert!(manager.elfreader_to_curreader(prog_reader) == CurReader::ProgReaders(0));
        assert!(prog_reader.id == 1);
        println!("Prog reader id = {}", prog_reader.id);
        println!("Test converter pass!");
    }
}