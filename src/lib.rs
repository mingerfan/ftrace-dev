mod utils;
mod ftrace;

// 由于libc的绑定比std的ffi更全，所以不使用ffi的c_char等类型
use std::ffi::CStr;
use libc::{c_int, c_char, c_uchar};

pub const RC_ERROR_CODE: c_int = -1;
pub const RC_SUCCESS_CODE: c_int = 0;
use ftrace::*;

#[no_mangle]
pub extern "C" fn add(left: usize, right: usize) -> usize {
    left + right
}

#[no_mangle]
pub extern "C" fn print_string(in_string: *const c_char, m_len: usize) -> c_int {
    let c_string = if !in_string.is_null() {
        let slice: &[u8] = unsafe { std::slice::from_raw_parts(in_string as *const c_uchar, m_len) };
        match CStr::from_bytes_until_nul(slice)  {
            Ok(s) => s,
            Err(_) => { 
                println!("Warning: m_len is less than string len!"); 
                return RC_ERROR_CODE;
            }
        }
    } else {
        println!("Warning: ptr is NULL!");
        return RC_ERROR_CODE;
    };
    let c_str_printable = match c_string.to_str() {
        Ok(s) => s,
        Err(_) => {
            println!("Invalid string");
            return RC_ERROR_CODE;
        }
    };
    println!("{}", c_str_printable);
    RC_SUCCESS_CODE
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
