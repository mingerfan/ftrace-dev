use super::elf_reader::*;
use crate::debug_println;
use core::panic;
use std::cell::{Ref, RefCell};
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use std::rc::Rc;
use std::cell::Cell;

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum CurReader {
    MainReader,
    ProgReaders(usize),
}

pub struct FuncInstance {
    // 这里instance的id主要是用于结合cur_reader定位函数信息位置的
    id: u32,
    reader: Option<CurReader>,
    func_type: FunType,
    ret_val: Cell<Option<(u64, Option<u64>)>>,
    paras: RefCell<Option<Vec<u64>>>,
    _start_time: u64,
    _end_time: Cell<u64>,
}

impl FuncInstance {
    fn new(id: u32, func_type: FunType, reader: CurReader, _start_time: u64, paras: Option<&Vec<u64>>) -> Self {
        FuncInstance {
            id,
            reader: Some(reader),
            func_type,
            ret_val: Cell::new(None),
            paras: RefCell::new(paras.cloned()),
            _start_time,
            _end_time: Cell::new(_start_time),
        }
    }

    fn new_with_nullreader(id: u32, _start_time: u64, paras: Option<&Vec<u64>>) -> Self {
        // 没有reader的函数一定时external的
        FuncInstance {
            id,
            reader: None,
            func_type: FunType::ExternalFunc,
            ret_val: Cell::new(None),
            paras: RefCell::new(paras.cloned()),
            _start_time,
            _end_time: Cell::new(_start_time),
        }
    }

    fn set_end_time(&self, end_time: u64) {
        self._end_time.set(end_time)
    }

    fn set_ret_val(&self, ret_val: Option<(u64, Option<u64>)>, show_context: bool) {
        if show_context {
            self.ret_val.set(ret_val);
        } else {
            self.ret_val.set(None);
        }
    }

    fn set_end_and_ret(&self, end_time: u64, ret_val: Option<(u64, Option<u64>)>, show_context: bool) {
        self.set_end_time(end_time);
        self.set_ret_val(ret_val, show_context);
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn reader(&self) -> Option<CurReader> {
        self.reader
    } 

    pub fn func_type(&self) -> FunType {
        self.func_type
    }
    #[allow(dead_code)]
    pub fn ret_val(&self) -> Option<(u64, Option<u64>)> {
        self.ret_val.get()
    }
    #[allow(dead_code)]
    pub fn paras(&self) -> Ref<Option<Vec<u64>>> {
        self.paras.borrow()
    }

    fn set_paras(&self, paras: Option<Vec<u64>>) {
        let mut paras_ = self.paras.borrow_mut();
        *paras_ = paras;
    }

    pub fn _start_time(&self) -> u64 {
        self._start_time
    }

    pub fn _end_time(&self) -> u64{
        self._end_time.get()
    }
}


pub struct Manager {
    show_context: bool,
    main_reader: ElfReader,
    cur_reader: CurReader,
    prog_readers: Option<Vec<ElfReader>>,
    trace_log: Vec<Rc<FuncInstance>>,
    time_base: Vec<u64>,
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
            time_base: Vec::new(),
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

    pub fn get_reader(&self, reader: &CurReader) -> &ElfReader {
        match *reader {
            CurReader::MainReader => &self.main_reader,
            CurReader::ProgReaders(x) => {
                let res = &self.prog_readers
                .as_ref()
                .expect("Option<Vec> is None, should not reach ProgReaders arm")[x];
                if res.id as usize == x + 1 {
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

    pub fn get_func_from_ins(&self, func_ins: &FuncInstance) -> Option<&Func> {
        if func_ins.func_type == FunType::ExternalFunc {
            return None;
        }
        self.func_reader(func_ins).and_then(|reader| {
            reader.get_func(func_ins.id)
        })
    }

    fn trace_log_push(&mut self, elem: Rc<FuncInstance>) {
        // 这是为了保证所有的trace_log被push进入元素的时候都携带一个时间戳
        self.trace_log.push(elem);
        self.time_base.push(self.get_time());
    }

    pub fn get_time_from_index(&self, idx: usize) -> u64 {
        self.time_base[idx]
    }

    pub fn get_time_base_end(&self) -> u64 {
        if let Some(time) = self.time_base.last() {
            time.to_owned()
        } else {
            0
        }
    }

    fn first_add_function(&mut self, pc: u64, paras: Option<&Vec<u64>>) {
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
        self.trace_log_push(func_info.clone());
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

    fn build_ins_and_push(&mut self, cur_reader: CurReader, pc: u64, paras: Option<&Vec<u64>>) {
        // 这里假设了已经找到了pc对应的reader
        let reader = self.get_reader(&cur_reader);
        let func = reader.find(pc);
        if let Some(named_func) = func {
            let func_ins = FuncInstance::new(named_func.id, named_func.func_type, 
                cur_reader, 
                self.get_time(), 
                paras);
            let func_ins = Rc::new(func_ins);
            self.trace_log_push(func_ins.clone());
            self.func_stack.push(func_ins);
        } else {
            // 如果没有找到，那就是匿名函数
            let func_ins = FuncInstance::new(0, FunType::ExternalFunc, 
                cur_reader, 
                self.get_time(), 
                paras);
            let func_ins = Rc::new(func_ins);
            if let Some(x) = self.trace_log.last() {
                if x.func_type == FunType::LocalFunc {
                    self.trace_log_push(func_ins.clone());
                }
            }
            // 由于是匿名函数，所以应该检查栈顶部是否是匿名函数
            // 如果是就不继续添加，不是就继续添加
            if let Some(x) = self.func_stack.last() {
                if x.func_type == FunType::LocalFunc {
                    self.func_stack.push(func_ins);
                }
            }
        }
        
    }
    
    fn noram_add_function(&mut self, pc: u64, paras: Option<&Vec<u64>>) {
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
        } else if self.prog_readers.is_none() {
            // 这里主要应对没有传入完整的elf的情况，保证可用性的判断
            let func_ins = FuncInstance::new_with_nullreader(0, self.get_time(), paras);
            let func_ins = Rc::new(func_ins);
            if let Some(x) = self.trace_log.last() {
                if x.func_type == FunType::LocalFunc {
                    self.trace_log_push(func_ins.clone());
                }
            }
            if let Some(x) = self.func_stack.last() {
                if x.func_type == FunType::LocalFunc {
                    self.func_stack.push(func_ins);
                }
            }
        } else {
            // 这里需要额外考虑没有传入progs reader但是有外部函数的情况
            let readers = self.prog_readers.as_ref()
            .expect("Unexpected behaviour!");
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
                debug_println!("The current pc: 0x{:X} does not have a compatible reader, 
                add as an anonymous function instance", pc);
                let func_ins = FuncInstance::new_with_nullreader(0, self.get_time(), paras);
                let func_ins = Rc::new(func_ins);
                if let Some(x) = self.trace_log.last() {
                    if x.func_type == FunType::LocalFunc {
                        self.trace_log_push(func_ins.clone());
                    }
                }
                if let Some(x) = self.func_stack.last() {
                    if x.func_type == FunType::LocalFunc {
                        self.func_stack.push(func_ins);
                    }
                }
            }
        }
    }

    // 这里的external和elf_reader的func vec的external意义不完全相同
    // 如果找不到就会标记external，所以manager的external算是func vec的external的超集
    pub fn jmp_check_add_function(&mut self, pc: u64, paras: Option<&Vec<u64>>) {
        if self.trace_log.is_empty() {
            assert!(self.func_stack.is_empty());
            self.first_add_function(pc, paras);
        } else {
            let last_func = self.trace_log.last()
            .expect("Last func is null, unexpected behaviour");
            if !self.check_bound(last_func, pc) {
                self.noram_add_function(pc, paras);
            }
        }
    }

    fn print_stack(&self) {
        if self.func_stack().len() >= 500 {
            return;
        }
        debug_println!("\n==========================cur stack===========================");
        for func_ins in self.func_stack() {
            if func_ins.func_type != FunType::ExternalFunc {
                let func = self.get_func_from_ins(func_ins).unwrap();
                debug_println!("function: {}, id: {}, ins_id: {}", func.name, func.id, func_ins.id);
            } else {
                debug_println!("function: unknown, ins_id: {}", func_ins.id);
            }
        }
    }

    // 这里的pc需要传入返回后的第一条指令的pc，返回值则是在ret的时候收集的
    pub fn ret_pop_function(&mut self, pc: u64, ret_val: Option<(u64, Option<u64>)>) {
        // Cell救我狗命

        let cur_func = self.trace_log.last()
        .expect("Ret must have current Function");
        cur_func.set_end_and_ret(self.get_time(), ret_val, self.show_context);

        let mut has_ext = false;
        let res = self.func_stack.iter().enumerate()
        .find(|(_, item)| {
            if item.func_type == FunType::ExternalFunc {
                has_ext |= true;
                return false;
            } 
            self.check_bound(item, pc)
        });

        if let Some((idx, target)) = res {
            if idx == self.func_stack.len() - 1 {
                self.print_stack();
                panic!("Ret target is on the top of ret stack, Unexpected behaviour");
            }
            let t_id = target.id;
            let t_reader = target.reader;
            let target = target.clone();
            while let Some(element) = self.func_stack.pop() {
                // 这里已经pop了，所以拿到的len就是对应刚才弹出的顶层元素的idx
                // 不能让返回到的那个函数pop，所以需要idx对应的前一个元素pop后就结束while
                if self.func_stack.len() == idx + 1 {
                    break;
                } else {
                    // 为弹出的函数设置end_time
                    element.set_end_time(self.get_time());
                    // 将弹出函数的参数设置为None，避免内存占用过大
                    element.set_paras(None);
                }
            }
            let elem = self.func_stack.last()
            .expect("Current stack should not empty");
            assert!(elem.id == t_id);
            assert!(elem.reader == t_reader);
            self.trace_log_push(target);
        } else if !has_ext {
            // 因为如果栈内没有外部函数，就不可能返回到区域外
            // 要么就是我写错了，要么就是有一些我不了解的机制
            // 这时候就直接panic了
            panic!("Unexpected behaviour, abort!");
        } else if self.trace_log.last()
            .expect("In ret, log can not be empty").func_type != FunType::ExternalFunc {
                // 此时需要记录一个External function
                // 为了简单起见，就不check reader了
                let func_ins = FuncInstance::new_with_nullreader(0, self.get_time(), None);
                let func_ins = Rc::new(func_ins);
                self.trace_log_push(func_ins)
        } // return
    }

    pub fn func_stack(&self) -> &Vec<Rc<FuncInstance>> {
        &self.func_stack
    }

    pub fn trace_log(&self) -> &Vec<Rc<FuncInstance>> {
        &self.trace_log
    }

}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use std::thread;
    use std::time::Duration;
    use std::fs::File;
    use std::io::Write;

    // const RED_START: &str = "\x1b[31m";
    // const RED_END: &str = "\x1b[0m";
    // const GREEN_START: &str = "\x1b[32m";
    // const GREEN_END: &str = "\x1b[0m";
    const BLUE_START: &str = "\x1b[34m";
    const BLUE_END: &str = "\x1b[0m";

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

    #[test]
    fn test_add_and_pop() {
        let mut manager = Manager::new(false, 
            "./test_elf/riscv64-nemu-interpreter", 
            Some(vec!["./test_elf/nanos-lite-riscv64-nemu.elf"]));
        let main_reader = &manager.main_reader.clone();
        let prog_reader = &manager.prog_readers.clone().unwrap()[0];
        let file = File::create("./target/log.txt").unwrap();

        println!("\n==========================To test add and pop==========================");
        // 测试main reader的函数调用
        for func in main_reader.func_vec().iter().skip(2) {
            manager.jmp_check_add_function(func.start, None);
            let func_ins = manager.func_stack.last().unwrap();
            let func_ins1 = manager.trace_log.last().unwrap();
            assert!(func_ins.id == func_ins1.id);
            assert!(func_ins.reader == func_ins1.reader);
            if func.func_type == FunType::LocalFunc && func.start != func.end {
                assert!(func_ins.id == func.id);
                assert!(func_ins.reader.unwrap() == CurReader::MainReader);
            } else if func.func_type == FunType::LocalFunc && func.start == func.end {
                assert!(func_ins.id == 0);
            } else {
                assert!(func_ins.reader.is_none());
                assert!(func_ins.func_type == FunType::ExternalFunc);
            }
            thread::sleep(Duration::from_micros(50));
        }

        fn print_log(manager: &Manager, mut file: &File) {
            if manager.trace_log.len() >= 3000 {
                return;
            }
            writeln!(file, "\n==========================cur vec==========================").unwrap();
            for (func_ins, time) in manager.trace_log
            .iter()
            .zip(manager.time_base.iter()) {
                if func_ins.func_type != FunType::ExternalFunc {
                    let func = manager.get_func_from_ins(func_ins).unwrap();
                    writeln!(file, "time: {}, function: {},\t \
                    ret_val: {:?},\t start_time: {}, end_time: {}", time, 
                    func.name,
                    func_ins.ret_val(),
                    func_ins._start_time(),
                    func_ins._end_time(),
                    ).unwrap();
                } else {
                    writeln!(file, "time: {}, unknown function", time).unwrap();
                }
            }
        }

        fn print_stack(manager: &Manager) {
            if manager.func_stack().len() >= 500 {
                return;
            }
            println!("\n==========================cur stack===========================");
            for func_ins in manager.func_stack() {
                if func_ins.func_type != FunType::ExternalFunc {
                    let func = manager.get_func_from_ins(func_ins).unwrap();
                    println!("function: {}, id: {}, ins_id: {}", func.name, func.id, func_ins.id);
                } else {
                    println!("function: unknown, ins_id: {}", func_ins.id);
                }
            }
        }

        fn pop(manager: &mut Manager, range: std::ops::Range<usize>) {
            for i in range.rev() {
                let func_ins = &manager.func_stack()[i].clone();
                let stack_len = manager.func_stack().len();
                if func_ins.func_type == FunType::LocalFunc {
                    let func = manager.get_func_from_ins(func_ins).unwrap();
                    // println!("Pop func: {}", func.name);
                    manager.ret_pop_function(func.start, None);
                    assert!(manager.func_stack().last().unwrap().id == func_ins.id);
                    assert!(manager.func_stack().last().unwrap().reader == func_ins.reader);
                    assert!(manager.trace_log.last().unwrap().id == func_ins.id);
                    assert!(manager.trace_log.last().unwrap().reader == func_ins.reader);
                } else {
                    // 这时候我们输入一个在栈中找不到的函数的地址
                    // 理论上来说，它不会弹出这个内容
                    manager.ret_pop_function(0x2710, None);
                    // println!("Pop none");
                    assert!(stack_len == manager.func_stack().len());
                    assert!(manager.func_stack().last().unwrap().id == 0);
                    assert!(manager.func_stack().last().unwrap().reader == func_ins.reader);
                    assert!(manager.trace_log.last().unwrap().id == 0);
                    assert!(manager.trace_log.last().unwrap().func_type == FunType::ExternalFunc);
                }

            }
        }
        // 这时候不能包括最后一个元素，因为它是当前正在运行的函数
        let range = 20..(manager.func_stack().len()-1);
        pop(&mut manager, range);

        // 以下三个函数是main reader内确定的三个函数，用于测试
        let func = manager.get_func_from_ins(&manager.func_stack()[2]).unwrap().to_owned();
        println!("{}Return to a function, id: {}, name: {} {}", BLUE_START, 
        func.id, if func.func_type == FunType::ExternalFunc { "unknown" } else { &func.name }, BLUE_END);
        manager.ret_pop_function(func.start, None);
        assert!(manager.func_stack().last().unwrap().id == func.id);
        assert!(manager.func_stack().last().unwrap().reader == Some(CurReader::MainReader));

        // 这个是一个空函数实例，找不到对应的函数
        // 直接分别用frame_dummy， register_tm_clones, deregister_tm_clones的起始地址进行测试
        // 此时stack应该一直保持在当前函数，不会弹出空函数实例
        // 但是log应该要记录这些东西
        let stack_len = manager.func_stack().len();
        let top_id = manager.func_stack().last().unwrap().id;
        manager.ret_pop_function(0x27C0, None);
        manager.ret_pop_function(0x2740, None);
        manager.ret_pop_function(0x2710, None);
        assert!(stack_len == manager.func_stack().len());
        assert!(manager.func_stack().last().unwrap().id == top_id);
        assert!(manager.func_stack().last().unwrap().reader == Some(CurReader::MainReader));
        assert!(manager.trace_log.last().unwrap().id == 0);
        assert!(manager.trace_log.last().unwrap().func_type == FunType::ExternalFunc);

        // 此时应该弹出到第一个函数，空函数实例也需要弹出
        let func = manager.get_func_from_ins(&manager.func_stack()[0]).unwrap().to_owned();
        println!("{}Return to a function, id: {}, name: {} {}", BLUE_START, 
        func.id, if func.func_type == FunType::ExternalFunc { "unknown" } else { &func.name }, BLUE_END);
        manager.ret_pop_function(func.start, None);
        assert!(manager.func_stack().last().unwrap().id == func.id);
        assert!(manager.func_stack().last().unwrap().reader == Some(CurReader::MainReader));

        print_stack(&manager);

        // 接下来是测试progs reader的调用
        println!("\n==========================Subtest: Progs reader==========================");
        for func in prog_reader.func_vec().iter() {
            manager.jmp_check_add_function(func.start, None);
            let func_ins = manager.func_stack.last().unwrap();
            let func_ins1 = manager.trace_log.last().unwrap();
            assert!(func_ins.id == func_ins1.id);
            assert!(func_ins.reader == func_ins1.reader);
            if func.func_type == FunType::LocalFunc && func.start != func.end {
                assert!(func_ins.id == func.id);
                assert!(func_ins.reader.unwrap() == CurReader::ProgReaders(0));
            } else if func.func_type == FunType::LocalFunc && func.start == func.end {
                assert!(func_ins.id == 0);
            } else {
                assert!(func_ins.reader.is_none());
                assert!(func_ins.func_type == FunType::ExternalFunc);
            }
            thread::sleep(Duration::from_micros(50));
        }

        // 这时候不能包括最后一个元素，因为它是当前正在运行的函数
        // 测试progs reader的弹出
        let range = 20..(manager.func_stack().len()-1);
        pop(&mut manager, range);

        // 测试多次调用同一个无法检测的函数prog reader的start
        manager.jmp_check_add_function(0x80000000, None);
        // 此时栈顶应该多一个空函数
        assert!(manager.func_stack().last().unwrap().id == 0);
        assert!(manager.func_stack().last().unwrap().func_type == FunType::ExternalFunc);
        assert!(manager.trace_log.last().unwrap().id == 0);
        assert!(manager.trace_log.last().unwrap().func_type == FunType::ExternalFunc);
        let stack_len = manager.func_stack().len();
        let log_len = manager.trace_log.len();
        manager.jmp_check_add_function(0x80000000, None);
        manager.jmp_check_add_function(0x80000000, None);
        manager.jmp_check_add_function(0x80000000, None);
        // 这时候都不应该添加新元素
        assert!(manager.func_stack().len() == stack_len);
        assert!(manager.trace_log.len() == log_len);

        // 添加一个新元素后测试不在所有reader的函数添加以及其返回
        manager.jmp_check_add_function(0x800013BC, None);
        
        // 不在所有reader内的函数
        manager.jmp_check_add_function(0x90000000, None);
        // 此时栈顶应该多一个空函数
        assert!(manager.func_stack().last().unwrap().id == 0);
        assert!(manager.func_stack().last().unwrap().func_type == FunType::ExternalFunc);
        assert!(manager.trace_log.last().unwrap().id == 0);
        assert!(manager.trace_log.last().unwrap().func_type == FunType::ExternalFunc);
        let stack_len = manager.func_stack().len();
        let log_len = manager.trace_log.len();
        manager.jmp_check_add_function(0x90000000, None);
        manager.jmp_check_add_function(0x80000000, None);
        manager.jmp_check_add_function(0x90000000, None);
        // 这时候都不应该添加新元素
        assert!(manager.func_stack().len() == stack_len);
        assert!(manager.trace_log.len() == log_len);

        // 测试unknown函数返回
        manager.ret_pop_function(0x800013BC, None);
        // 简单测试一下就可以了
        assert!(manager.get_func_from_ins(manager.func_stack()
            .last().unwrap()).unwrap().name == "memset");
        assert!(manager.get_func_from_ins(manager.trace_log
            .last().unwrap()).unwrap().name == "memset");
        assert!(manager.func_stack().last().unwrap().func_type == FunType::LocalFunc);
        assert!(manager.trace_log.last().unwrap().func_type == FunType::LocalFunc);

        print_stack(&manager);

        // 再测试直接返回main reader
        manager.ret_pop_function(0x2705, None); // _start
        assert!(manager.get_func_from_ins(manager.func_stack()
            .last().unwrap()).unwrap().name == "_start");
        assert!(manager.get_func_from_ins(manager.trace_log
            .last().unwrap()).unwrap().name == "_start");

        print_stack(&manager);

        print_log(&manager, &file);
        
        let func_ins_size = std::mem::size_of::<FuncInstance>();
        println!("Sizeof FuncInstance: {}", func_ins_size);
        println!("Current Log memory used by elements is: {}, memory allocated is: {}", 
        manager.trace_log.len() * func_ins_size, manager.trace_log.capacity() * func_ins_size);

        // 接下来是压力测试，用于测试大数据下的内存占用
        println!("\n==========================Stress testing==========================");
        for _ in 0..500_000 {
            manager.jmp_check_add_function(0x800013BC, None);
            manager.jmp_check_add_function(0x4510, None);
        }
        println!("Current Log memory used by elements is: {}, memory allocated is: {}", 
        manager.trace_log.len() * func_ins_size, manager.trace_log.capacity() * func_ins_size);
        // 这时候千万不能print_log，会爆炸的
    }
}