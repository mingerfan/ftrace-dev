#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define RC_ERROR_CODE -1

#define RC_SUCCESS_CODE 0

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

uintptr_t add(uintptr_t left, uintptr_t right);

int print_string(const char *in_string, uintptr_t m_len);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus
