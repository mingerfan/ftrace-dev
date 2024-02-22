
use std::{cell::RefCell, sync::Mutex};
use crate::manager::*;
use std::collections::HashSet;
use bitpattern::bitpattern;


// 这里用了unsafe，实际上我不会在任何多线程来修改这些数据
// 当然，c语言侧也需要保证是单线程的


struct ManagerBuilder {
    show_context: bool,
    main_path: String,
    progs_path: Option<HashSet<String>>
}

#[derive(PartialEq, Eq)]
enum ImmType {
    I,
    J,
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

fn sign_extend_to_u64(value: u64, bit_width: u8) -> u64 {
    // 检查位宽是否有效（1至64之间，因为我们扩展到64位）
    if bit_width == 0 || bit_width > 64 {
        panic!("bit_width must be between 1 and 64");
    }
    
    // 如果位宽已经是64位，直接返回值
    if bit_width == 64 {
        return value;
    }
    
    // 创建一个掩码，它将在原始数值的符号位上有一个单一的1，其余位都是0。
    let mask = 1u64 << (bit_width - 1);
    
    // 检查符号位是否被设置（是否为负数）
    if value & mask == 0 {
        // 如果符号位为0，直接返回值，因为没有符号扩展的需要
        value
    } else {
        // 如果符号位为1，执行符号扩展：
        // 将掩码的所有位取反得到一个新掩码，这个新掩码将用于生成符号扩展位
        // 然后通过或操作(|)将这些扩展位添加到原始值上
        let sign_ext = !((1u64 << bit_width) - 1);
        value | sign_ext
    }
}

fn bits(value: u64, a: u8, b: u8) -> u64 {
    if a < b || a > 63 {
        panic!("Invalid range: a must be greater than or equal to b, and a must be less than 64.");
    }
    
    // 创建一个掩码，它在位于 a 和 b 之间的每一位上都是1
    let mask = ((1u64 << (a - b + 1)) - 1) << b;

    // 应用掩码，然后右移 b 位
    (value & mask) >> b
}


fn bitmask(bits: u8) -> u64 {
    (1u64 << bits) - 1
}

#[allow(dead_code)]
#[cfg(test)]
fn check_instruction_print(inst: u32) {
    if bitpattern!("???????_?????_?????_???_?????_11011_11", inst).is_some() {
        // jal
        println!("Match jal!");
    } else if bitpattern!("???????_?????_?????_000_?????_11001_11", inst).is_some() {
        // jalr
        println!("Match jalr!");
    }
}

fn get_imm(inst: u32, imm_type: ImmType) -> u64 {
    let i = inst as u64;
    match imm_type {
        ImmType::I => {
            sign_extend_to_u64(bits(i, 31, 20), 12)
        }
        ImmType::J => {
            sign_extend_to_u64(bits(i, 31, 31), 1) << 20 |
            bits(i, 19, 12) << 12    |
            bits(i, 20, 20) << 11    |
            bits(i, 30, 25) << 5     |
            bits(i, 24, 21) << 1
        }
        _ => panic!()
    }
}


pub fn check_instruction(pc: u64, inst: u32, regs: &[u64]) {
    // 这里的pc是当前指令的pc，通过这个来计算出来跳转到的地址
    let target_pc = if bitpattern!("???????_?????_?????_???_?????_11011_11", inst).is_some() {
        // jal
        let immj = get_imm(inst, ImmType::J);
        immj + pc
    } else if bitpattern!("???????_?????_?????_000_?????_11001_11", inst).is_some() {
        // jalr
        let immi = get_imm(inst, ImmType::I);
        (immi + regs[bits(inst as u64, 19, 15) as usize]) & !(bitmask(1))
    } else {
        panic!("Unexpected behaviour")
    };
    G_MANAGER.with(|elem| {
        let mut manager = elem.borrow_mut();
        if let Some(ref mut manager) = *manager {
            let inst = inst as u64;
            if (bits(inst, 19, 15) == 1) && (bits(inst, 11, 7) == 0) {
                // 首先判断是否是return
                // riscv用x10和x11返回值
                manager.ret_pop_function(target_pc, Some((regs[10], Some(regs[11]))));
            } else {
                // 这里对于Paras的参数设计有问题，应该直接要求顶层传入有所有权的内容
                // 只能降低效率了
                let regs = regs.to_owned();
                manager.jmp_check_add_function(target_pc, Some(&regs));
            }
        }
    });
}

#[allow(dead_code)]
#[cfg(test)]
fn target_pc_gen(pc: u64, inst: u32, regs: &[u64]) -> u64 {
    if bitpattern!("???????_?????_?????_???_?????_11011_11", inst).is_some() {
        // jal
        let immj = get_imm(inst, ImmType::J);
        immj + pc
    } else if bitpattern!("???????_?????_?????_000_?????_11001_11", inst).is_some() {
        // jalr
        let immi = get_imm(inst, ImmType::I);
        (immi + regs[bits(inst as u64, 19, 15) as usize]) & !(bitmask(1))
    } else {
        panic!("Unexpected behaviour")
    }
} 

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