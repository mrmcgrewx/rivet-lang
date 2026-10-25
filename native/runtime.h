#ifndef RIVET_RUNTIME_H
#define RIVET_RUNTIME_H

#include <stdint.h>

void *rv_text(const unsigned char *bytes, uint64_t length);
void *rv_concat(void *const *values, uint64_t count);
void *rv_i64_text(int64_t value);
void *rv_bool_text(uint8_t value);
void *rv_json_text(void *text);
void *rv_json_i64(int64_t value);
void *rv_json_bool(uint8_t value);
void *rv_object(const unsigned char *const *keys, const uint64_t *key_lengths,
                void *const *json_values, uint64_t count);
void rv_print(void *text);
_Noreturn void rv_fail(const unsigned char *message, uint64_t length);
void rv_add_route(const unsigned char *path, uint64_t length, uint32_t status, void *json);
int32_t rv_serve(uint32_t port);
void rv_cleanup(void);

#endif
