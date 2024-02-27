mod utils;
mod ftrace;

// 由于libc的绑定比std的ffi更全，所以不使用ffi的c_char等类型
use std::ffi::CStr;
use libc::{c_char, c_uchar };

pub const RC_ERROR_CODE: isize = -1;
pub const RC_SUCCESS_CODE: isize = 0;
pub const MAX_PATH_LEN: usize = 300;

#[no_mangle]
pub extern "C" fn add(left: usize, right: usize) -> usize {
    left + right
}

// 简单的字符串复制可以用这种方法
fn get_string(in_string: *const c_char, m_len: usize) -> Result<String, isize> {
    let c_string = if !in_string.is_null() {
        let slice: &[u8] = unsafe { std::slice::from_raw_parts(in_string as *const c_uchar, m_len) };
        match CStr::from_bytes_until_nul(slice)  {
            Ok(s) => s,
            Err(_) => { 
                println!("Warning: m_len is less than string len!"); 
                return Err(RC_ERROR_CODE);
            }
        }
    } else {
        println!("Warning: ptr is NULL!");
        return Err(RC_ERROR_CODE);
    };
    let c_str_printable = match c_string.to_str() {
        Ok(s) => s,
        Err(_) => {
            println!("Invalid string");
            return Err(RC_ERROR_CODE);
        }
    };
    Ok(c_str_printable.to_string())
}

#[no_mangle]
pub extern "C" fn print_string(in_string: *const c_char, m_len: usize) -> isize {
    let c_str_printable = get_string(in_string, m_len);
    if let Ok(r_str) = c_str_printable {
        println!("{}", r_str);
        RC_SUCCESS_CODE
    } else {
        RC_ERROR_CODE
    }
}

#[no_mangle]
pub extern "C" fn start_builder(main_path: *const c_char) -> isize {
    let string = get_string(main_path, MAX_PATH_LEN);
    if let Ok(main_path) = string {
        if ftrace::start_builder(&main_path).is_ok() {
            RC_SUCCESS_CODE
        } else {
            RC_ERROR_CODE
        }
    } else {
        RC_ERROR_CODE
    }
}

#[no_mangle]
pub extern "C" fn set_show_context(show_context: bool) ->isize {
    if ftrace::set_show_context(show_context).is_ok() {
        RC_SUCCESS_CODE
    } else {
        RC_ERROR_CODE
    }
}

#[no_mangle]
pub extern "C" fn add_prog_path(path: *const c_char) -> isize {
    if let Ok(path) = get_string(path, MAX_PATH_LEN) {
        if ftrace::add_prog_path(path).is_ok() {
            RC_SUCCESS_CODE
        } else {
            RC_ERROR_CODE
        }
    } else {
        RC_ERROR_CODE
    }
}

#[no_mangle]
pub extern "C" fn build_builder() -> isize {
    if ftrace::build_builder().is_ok() {
        RC_SUCCESS_CODE
    } else {
        RC_ERROR_CODE
    }
}

#[no_mangle]
// 这里有一个假设，就是只传入32个寄存器，不能多不能少
pub extern "C" fn check_instruction(pc: u64, inst: u32, regs: *const u64) -> isize {
    if !regs.is_null() {
        let slice: &[u64] = unsafe { std::slice::from_raw_parts(regs, 32) };
        ftrace::check_instruction(pc, inst, slice);
        RC_SUCCESS_CODE
    } else {
        RC_ERROR_CODE
    }
}

#[no_mangle]
pub extern "C" fn print_stack(path: *const c_char) -> isize {
    if let Ok(path) = get_string(path, MAX_PATH_LEN) {
        if ftrace::print_stack(path).is_ok() {
            RC_SUCCESS_CODE
        } else {
            RC_ERROR_CODE
        } 
    } else {
        RC_ERROR_CODE
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn print() {
        let c_string = CString::new("Hello").expect("Failed!");
        assert_eq!(print_string(c_string.as_ptr(), 20), RC_SUCCESS_CODE)
    }
}
