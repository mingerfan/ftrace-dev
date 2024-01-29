这里做一个规定，对于
- bool => bool
- char => uint32_t
- u8 => uint8_t
- u16 => uint16_t
- u32 => uint32_t
- u64 => uint64_t
- usize => uintptr_t
- i8 => int8_t
- i16 => int16_t
- i32 => int32_t
- i64 => int64_t
- isize => intptr_t
- f32 => float
- f64 => double
除了某些时候需要明确接口的C类型，都采用rust std的类型作为接口
但是对于
- c_void => void
- c_char => char
- c_schar => signed char
- c_uchar => unsigned char
都采用libc的类型作为接口
