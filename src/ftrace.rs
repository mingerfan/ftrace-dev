
use std::{borrow::BorrowMut, cell::RefCell, sync::Mutex};
use crate::manager::{self, *};
use std::collections::HashSet;
use bitpattern::bitpattern;


// 这里用了unsafe，实际上我不会在任何多线程来修改这些数据
// 当然，c语言侧也需要保证是单线程的


struct ManagerBuilder {
    show_context: bool,
    main_path: String,
    progs_path: Option<HashSet<String>>
}

thread_local! {
    static G_MANAGER: RefCell<Option<Manager>> = RefCell::new(None);
}

static G_BUILDER: Mutex<Option<ManagerBuilder>> = Mutex::new(None);


pub fn start_builder(main_path: &str) {
    static IS_INIT: Mutex<bool> = Mutex::new(false);

    if !(*IS_INIT.lock().unwrap()) {
        // false就初始化
        let mut data = G_BUILDER.lock().unwrap();
        *data = Some(ManagerBuilder {
            show_context: false,
            main_path: main_path.to_string(),
            progs_path: None
        });
    } else {
        println!("Builder is constructed!");
    }
}

pub fn set_show_context(show_context: bool) {
    let mut data = G_BUILDER.lock().unwrap();
    if let Some(x) = data.as_mut() {
        x.show_context = show_context;
    } else {
        println!("Warning: current builder is NULL!");
    }
}

pub fn add_prog_path(path: String) {
    let mut data = G_BUILDER.lock().unwrap();
    if let Some(x) = data.as_mut() {
        if let Some(progs_path) = x.progs_path.as_mut() {
            progs_path.insert(path.to_string());
        } else {
            let mut set = HashSet::new();
            set.insert(path);
            x.progs_path = Some(set);
        }
    } else {
        println!("Warning: current builder is NULL");
    }
}

pub fn build_builder() {
    // 贼难写这一部分，主要是Manager的接口设计的有问题
    let mut builder = G_BUILDER.lock().unwrap();
    if let Some(builder) = builder.as_mut() {
        G_MANAGER.with(|f| {
            let mut manager = f.borrow_mut();
            if manager.is_none() {
                let progs_path = builder.progs_path.clone();
                let manager_new = if let Some(set) = progs_path {
                    let progs = Some(set.iter().map(|x| x.as_str()).collect::<Vec<&str>>());
                    Manager::new(builder.show_context, &builder.main_path, progs)
                } else {
                    Manager::new(builder.show_context, &builder.main_path, None)
                };
                *manager = Some(manager_new);
            } else {
                println!("Warning: manager is initialized!");
            }
        })
    } else {
        println!("Warning: current builder is NULL");
    }
}

#[allow(dead_code)]
fn check_instruction_print(inst: u32) {
    if bitpattern!("???????_?????_?????_???_?????_11011_11", inst).is_some() {
        // jal
        println!("Match jal!");
    } else if bitpattern!("???????_?????_?????_000_?????_11001_11", inst).is_some() {
        // jalr
        println!("Match jalr!");
    }
}

fn bit_slice(n: u32, high: u8, low: u8) -> u32 {
    if high < low {
        panic!("high must be greater than or equal to low");
    }
    (n >> low) & ((1 << (high - low + 1)) - 1)
}

// pub fn check_instruction(pc: u64, inst: u32) {
//     let target_pc = if bitpattern!("???????_?????_?????_???_?????_11011_11", inst).is_some() {
//         // jal
//         let immj = 
//     } else if bitpattern!("???????_?????_?????_000_?????_11001_11", inst).is_some() {
//         // jalr
        
//     }
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_check_instruction() {
        // c81ff0ef jal
        check_instruction_print(0xc81ff0ef);
        // 000780e7 jalr
        check_instruction_print(0x000780e7);
    }
}