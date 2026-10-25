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

/* RVB2 computation support. Control flow and arithmetic are generated code. */
int64_t rv2_read_int(void);
void *rv2_read_array(void);
int64_t rv2_array_len(void *array);
int64_t rv2_array_get(void *array, int64_t index);
void *rv2_array_make(const int64_t *values, uint64_t count);
void *rv2_map_new(void);
int64_t rv2_map_get(void *map, int64_t key, int64_t fallback);
void rv2_map_put(void *map, int64_t key, int64_t value);
void rv2_tick(void);
void rv2_return(void *array);
void *rv2_read_list(void);
void *rv2_list_cons(int64_t value, void *tail);
int64_t rv2_list_value(void *list);
void *rv2_list_next(void *list);
int64_t rv2_list_len(void *list);
void rv2_return_list(void *list);
void rv2_return_float(double value);

#endif
