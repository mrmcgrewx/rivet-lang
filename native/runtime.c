/* Native services for directly generated Rivet code. No program parser or VM. */
#define _POSIX_C_SOURCE 200809L
#include "runtime.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <limits.h>
#include <math.h>
#include <poll.h>
#include <signal.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

#define RV_MAX_VALUE ((size_t)65536)
#define RV_MAX_HEADERS ((size_t)8192)
#define RV_MAX_BODY ((size_t)524288)
#define RV_MAX_PATH ((size_t)2048)
#define RV_MAX_ROUTES ((size_t)32)
#define RV2_MAX_INPUT ((size_t)262144)
#define RV2_MAX_ITEMS ((size_t)10000)
#define RV2_MAP_CAPACITY ((size_t)32768)
#define RV2_MAX_ALLOCATION ((size_t)67108864)
#define RV2_MAX_STEPS UINT64_C(1000000)

typedef enum { RV_TEXT, RV_JSON, RV_INT_ARRAY, RV_INT_MAP, RV_INT_LIST } rv_kind;
typedef struct rv_value {
    struct rv_value *next;
    struct rv_value *json_encoding;
    size_t length;
    rv_kind kind;
    _Alignas(max_align_t) unsigned char bytes[];
} rv_value;
typedef struct {
    rv_value *path;
    rv_value *body;
    uint32_t status;
} rv_route;
static rv_value *rv_allocations;
static rv_route rv_routes[RV_MAX_ROUTES];
static size_t rv_route_count;
static volatile sig_atomic_t rv_stopping;
static unsigned char *rv2_input;
static size_t rv2_input_length;
static size_t rv2_input_cursor;
static size_t rv2_allocation_charge;
static uint64_t rv2_steps;

typedef struct {
    int64_t key;
    int64_t value;
    uint64_t occupied;
} rv2_map_slot;
typedef struct {
    size_t count;
    rv2_map_slot slots[RV2_MAP_CAPACITY];
} rv2_map;
typedef struct {
    int64_t value;
    rv_value *tail;
    uint64_t length;
} rv2_list_node;

_Noreturn void rv_fail(const unsigned char *message, uint64_t length) {
    static const char prefix[] = "Rivet runtime: ";
    (void)fwrite(prefix, 1, sizeof(prefix) - 1, stderr);
    if (message != NULL && length <= SIZE_MAX) {
        (void)fwrite(message, 1, (size_t)length, stderr);
    }
    (void)fputc('\n', stderr);
    rv_cleanup();
    exit(EXIT_FAILURE);
}

static _Noreturn void rv_error(const char *message) {
    rv_fail((const unsigned char *)message, (uint64_t)strlen(message));
}

static _Noreturn void rv_system_error(const char *context) {
    char message[256];
    int saved_errno = errno;
    (void)snprintf(message, sizeof(message), "%s: %s", context, strerror(saved_errno));
    rv_error(message);
}

static size_t rv_checked_size(size_t size, const char *context) {
    if (size > RV_MAX_VALUE) {
        char message[160];
        (void)snprintf(message, sizeof(message), "%s exceeds 65536 bytes (%zu bytes required)",
                       context, size);
        rv_error(message);
    }
    return size;
}

static rv_value *rv_allocate(size_t length, rv_kind kind) {
    if (length > SIZE_MAX - sizeof(rv_value)) rv_error("allocation size overflow");
    rv_value *value = malloc(sizeof(*value) + length);
    if (value == NULL) rv_error("cannot allocate native value");
    value->next = rv_allocations;
    value->json_encoding = NULL;
    value->length = length;
    value->kind = kind;
    rv_allocations = value;
    return value;
}

static rv_value *rv_require(void *pointer, rv_kind kind) {
    rv_value *value = pointer;
    if (value == NULL || value->kind != kind) rv_error("invalid native value type");
    return value;
}

void *rv_text(const unsigned char *bytes, uint64_t length) {
    if (length > RV_MAX_VALUE) rv_error("computed Text exceeds 65536 bytes");
    if (length != 0 && bytes == NULL) rv_error("missing text bytes");
    rv_value *value = rv_allocate((size_t)length, RV_TEXT);
    if (length != 0) memcpy(value->bytes, bytes, (size_t)length);
    return value;
}

void *rv_concat(void *const *values, uint64_t count) {
    if (count > SIZE_MAX || (count != 0 && values == NULL)) rv_error("invalid concat operands");
    size_t length = 0;
    for (size_t i = 0; i < (size_t)count; ++i) {
        rv_value *value = rv_require(values[i], RV_TEXT);
        length = rv_checked_size(length + value->length, "computed Text");
    }
    rv_value *result = rv_allocate(length, RV_TEXT);
    size_t offset = 0;
    for (size_t i = 0; i < (size_t)count; ++i) {
        rv_value *value = values[i];
        if (value->length != 0) memcpy(result->bytes + offset, value->bytes, value->length);
        offset += value->length;
    }
    return result;
}

void *rv_i64_text(int64_t value) {
    unsigned char bytes[32];
    int length = snprintf((char *)bytes, sizeof(bytes), "%" PRId64, value);
    if (length < 0 || (size_t)length >= sizeof(bytes)) rv_error("cannot format signed integer");
    return rv_text(bytes, (uint64_t)length);
}

void *rv_bool_text(uint8_t value) {
    return value ? rv_text((const unsigned char *)"true", 4)
                 : rv_text((const unsigned char *)"false", 5);
}

static size_t rv_quoted_length(const unsigned char *bytes, size_t length) {
    size_t encoded = 2;
    for (size_t i = 0; i < length; ++i) {
        unsigned char c = bytes[i];
        if (c == '"' || c == '\\' || c == '\b' || c == '\f' || c == '\n' || c == '\r' || c == '\t') {
            encoded += 2;
        } else {
            encoded += c < 0x20 ? 6 : 1;
        }
    }
    return encoded;
}

static size_t rv_quote(unsigned char *out, const unsigned char *bytes, size_t length) {
    static const unsigned char hex[] = "0123456789abcdef";
    size_t cursor = 0;
    out[cursor++] = '"';
    for (size_t i = 0; i < length; ++i) {
        unsigned char c = bytes[i];
        unsigned char escape = 0;
        switch (c) {
            case '"': escape = '"'; break;
            case '\\': escape = '\\'; break;
            case '\b': escape = 'b'; break;
            case '\f': escape = 'f'; break;
            case '\n': escape = 'n'; break;
            case '\r': escape = 'r'; break;
            case '\t': escape = 't'; break;
            default: break;
        }
        if (escape != 0) {
            out[cursor++] = '\\';
            out[cursor++] = escape;
        } else if (c < 0x20) {
            out[cursor++] = '\\';
            out[cursor++] = 'u';
            out[cursor++] = '0';
            out[cursor++] = '0';
            out[cursor++] = hex[c >> 4];
            out[cursor++] = hex[c & 15];
        } else {
            out[cursor++] = c;
        }
    }
    out[cursor++] = '"';
    return cursor;
}

void *rv_json_text(void *text) {
    rv_value *value = rv_require(text, RV_TEXT);
    if (value->json_encoding != NULL) return value->json_encoding;
    /* This temporary encoding can exceed 64 KiB. The containing object checks
       its complete serialized size, in field order, as the interpreter does. */
    rv_value *result = rv_allocate(rv_quoted_length(value->bytes, value->length), RV_JSON);
    (void)rv_quote(result->bytes, value->bytes, value->length);
    value->json_encoding = result;
    return result;
}

void *rv_json_i64(int64_t value) {
    rv_value *result = rv_i64_text(value);
    result->kind = RV_JSON;
    return result;
}

void *rv_json_bool(uint8_t value) {
    rv_value *result = rv_bool_text(value);
    result->kind = RV_JSON;
    return result;
}

typedef struct {
    const unsigned char *key;
    size_t length;
    rv_value *value;
    size_t original_index;
} rv_field;

static int rv_compare_fields(const void *left_pointer, const void *right_pointer) {
    const rv_field *left = left_pointer;
    const rv_field *right = right_pointer;
    size_t common = left->length < right->length ? left->length : right->length;
    int order = common == 0 ? 0 : memcmp(left->key, right->key, common);
    if (order != 0) return order;
    if (left->length != right->length) return left->length < right->length ? -1 : 1;
    return (left->original_index > right->original_index) -
           (left->original_index < right->original_index);
}

static int rv_same_key(const rv_field *left, const rv_field *right) {
    return left->length == right->length &&
           (left->length == 0 || memcmp(left->key, right->key, left->length) == 0);
}

void *rv_object(const unsigned char *const *keys, const uint64_t *key_lengths,
                void *const *json_values, uint64_t count) {
    if (count > RV_MAX_VALUE ||
        (count != 0 && (keys == NULL || key_lengths == NULL || json_values == NULL))) {
        rv_error("invalid object fields");
    }
    /* Keep temporary indexing in the allocation list so failure frees it too. */
    rv_value *storage = rv_allocate((size_t)count * sizeof(rv_field), RV_TEXT);
    rv_field *fields = (rv_field *)(void *)storage->bytes;
    for (size_t i = 0; i < (size_t)count; ++i) {
        if (key_lengths[i] > RV_MAX_VALUE || (key_lengths[i] != 0 && keys[i] == NULL)) {
            rv_error("invalid object key");
        }
        fields[i] = (rv_field){keys[i], (size_t)key_lengths[i],
                              rv_require(json_values[i], RV_JSON), i};
    }
    qsort(fields, (size_t)count, sizeof(*fields), rv_compare_fields);
    size_t length = 2;
    size_t unique = 0;
    for (size_t i = 0; i < (size_t)count; ++i) {
        if (i + 1 < (size_t)count && rv_same_key(&fields[i], &fields[i + 1])) continue;
        length = rv_checked_size(length + (unique != 0 ? 1 : 0) +
                                 rv_quoted_length(fields[i].key, fields[i].length) + 1 +
                                 fields[i].value->length, "computed Json");
        ++unique;
    }
    rv_value *result = rv_allocate(length, RV_JSON);
    size_t offset = 0;
    result->bytes[offset++] = '{';
    for (size_t i = 0; i < (size_t)count; ++i) {
        if (i + 1 < (size_t)count && rv_same_key(&fields[i], &fields[i + 1])) continue;
        if (offset > 1) result->bytes[offset++] = ',';
        offset += rv_quote(result->bytes + offset, fields[i].key, fields[i].length);
        result->bytes[offset++] = ':';
        memcpy(result->bytes + offset, fields[i].value->bytes, fields[i].value->length);
        offset += fields[i].value->length;
    }
    result->bytes[offset] = '}';
    return result;
}

static void rv_ignore_sigpipe(void) {
    struct sigaction action;
    memset(&action, 0, sizeof(action));
    action.sa_handler = SIG_IGN;
    (void)sigemptyset(&action.sa_mask);
    if (sigaction(SIGPIPE, &action, NULL) != 0) rv_system_error("cannot ignore SIGPIPE");
}

void rv_print(void *text) {
    rv_value *value = rv_require(text, RV_TEXT);
    (void)rv_checked_size(value->length + 1, "console output including newline");
    rv_ignore_sigpipe();
    if (fwrite(value->bytes, 1, value->length, stdout) != value->length ||
        fputc('\n', stdout) == EOF || fflush(stdout) != 0) {
        rv_system_error("cannot write console output");
    }
}

/* RVB2's logical quota is defined by the language, independent of allocator
   headers and the reference implementation's container representation. */
static void rv2_charge(size_t bytes) {
    if (bytes > RV2_MAX_ALLOCATION - rv2_allocation_charge) {
        rv_error("compute allocation budget exceeds 67108864 bytes");
    }
    rv2_allocation_charge += bytes;
}

void rv2_tick(void) {
    if (rv2_steps == RV2_MAX_STEPS) rv_error("compute instruction limit exceeded (1000000)");
    ++rv2_steps;
}

uint64_t *rv2_fuel_counter(void) {
    return &rv2_steps;
}

static int rv2_space(unsigned char byte) {
    return byte == ' ' || (byte >= '\t' && byte <= '\r');
}

static void rv2_load_input(void) {
    if (rv2_input != NULL) return;
    rv2_input = malloc(RV2_MAX_INPUT + 1);
    if (rv2_input == NULL) rv_error("cannot allocate runtime input");
    rv2_input_length = fread(rv2_input, 1, RV2_MAX_INPUT + 1, stdin);
    if (ferror(stdin)) rv_system_error("cannot read runtime input");
    if (rv2_input_length > RV2_MAX_INPUT) rv_error("runtime input exceeds 262144 bytes");
}

static void rv2_skip_space(void) {
    while (rv2_input_cursor < rv2_input_length && rv2_space(rv2_input[rv2_input_cursor])) {
        ++rv2_input_cursor;
    }
}

int64_t rv2_read_int(void) {
    rv2_load_input();
    rv2_skip_space();
    if (rv2_input_cursor == rv2_input_length) rv_error("runtime input is missing an integer");
    int negative = rv2_input[rv2_input_cursor] == '-';
    if (negative) ++rv2_input_cursor;
    if (rv2_input_cursor == rv2_input_length || rv2_input[rv2_input_cursor] < '0' ||
        rv2_input[rv2_input_cursor] > '9') {
        rv_error("runtime input contains an invalid signed integer");
    }
    uint64_t magnitude = 0;
    uint64_t limit = negative ? UINT64_C(9223372036854775808) : UINT64_C(9223372036854775807);
    while (rv2_input_cursor < rv2_input_length) {
        unsigned char byte = rv2_input[rv2_input_cursor];
        if (byte < '0' || byte > '9') break;
        uint64_t digit = (uint64_t)(byte - '0');
        if (magnitude > (limit - digit) / 10) rv_error("runtime input integer exceeds signed i64 range");
        magnitude = magnitude * 10 + digit;
        ++rv2_input_cursor;
    }
    if (rv2_input_cursor < rv2_input_length && !rv2_space(rv2_input[rv2_input_cursor])) {
        rv_error("runtime input contains an invalid signed integer");
    }
    if (negative && magnitude == UINT64_C(9223372036854775808)) return INT64_MIN;
    return negative ? -(int64_t)magnitude : (int64_t)magnitude;
}

static rv_value *rv2_new_array(uint64_t count) {
    if (count > RV2_MAX_ITEMS) rv_error("integer array length exceeds 10000");
    size_t bytes = (size_t)count * sizeof(int64_t);
    rv2_charge(bytes);
    return rv_allocate(bytes, RV_INT_ARRAY);
}

void *rv2_read_array(void) {
    int64_t count = rv2_read_int();
    if (count < 0) rv_error("integer array length must be nonnegative");
    rv_value *result = rv2_new_array((uint64_t)count);
    int64_t *items = (int64_t *)(void *)result->bytes;
    for (size_t index = 0; index < (size_t)count; ++index) items[index] = rv2_read_int();
    return result;
}

void *rv2_array_make(const int64_t *values, uint64_t count) {
    if (count != 0 && values == NULL) rv_error("missing integer array elements");
    rv_value *result = rv2_new_array(count);
    if (count != 0) memcpy(result->bytes, values, result->length);
    return result;
}

int64_t rv2_array_len(void *array) {
    rv_value *value = rv_require(array, RV_INT_ARRAY);
    return (int64_t)(value->length / sizeof(int64_t));
}

int64_t rv2_array_get(void *array, int64_t index) {
    rv_value *value = rv_require(array, RV_INT_ARRAY);
    if (index < 0 || (uint64_t)index >= value->length / sizeof(int64_t)) {
        rv_error("integer array index out of bounds");
    }
    return ((const int64_t *)(const void *)value->bytes)[(size_t)index];
}

void *rv2_map_new(void) {
    rv2_charge(RV2_MAP_CAPACITY * 24);
    rv_value *value = rv_allocate(sizeof(rv2_map), RV_INT_MAP);
    memset(value->bytes, 0, sizeof(rv2_map));
    return value;
}

/* SplitMix64's finalizer spreads all signed keys, including zero and extrema.
   Unsigned arithmetic is intentional and defined modulo 2^64. */
static size_t rv2_hash(int64_t key) {
    uint64_t value = (uint64_t)key;
    value = (value ^ (value >> 30)) * UINT64_C(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)) * UINT64_C(0x94d049bb133111eb);
    value ^= value >> 31;
    return (size_t)(value & (RV2_MAP_CAPACITY - 1));
}

static rv2_map_slot *rv2_find_slot(rv2_map *map, int64_t key) {
    size_t index = rv2_hash(key);
    for (size_t probes = 0; probes < RV2_MAP_CAPACITY; ++probes) {
        rv2_map_slot *slot = &map->slots[index];
        if (!slot->occupied || slot->key == key) return slot;
        index = (index + 1) & (RV2_MAP_CAPACITY - 1);
    }
    rv_error("integer map has no available slot");
}

int64_t rv2_map_get(void *map, int64_t key, int64_t fallback) {
    rv_value *value = rv_require(map, RV_INT_MAP);
    rv2_map_slot *slot = rv2_find_slot((rv2_map *)(void *)value->bytes, key);
    return slot->occupied ? slot->value : fallback;
}

void rv2_map_put(void *map, int64_t key, int64_t item) {
    rv_value *value = rv_require(map, RV_INT_MAP);
    rv2_map *table = (rv2_map *)(void *)value->bytes;
    rv2_map_slot *slot = rv2_find_slot(table, key);
    if (!slot->occupied) {
        if (table->count == RV2_MAX_ITEMS) rv_error("integer map exceeds 10000 unique keys");
        ++table->count;
        slot->key = key;
        slot->occupied = 1;
    }
    slot->value = item;
}

static void rv2_finish_input(void) {
    rv2_load_input();
    rv2_skip_space();
    if (rv2_input_cursor != rv2_input_length) rv_error("runtime input has unconsumed values");
}

static void rv2_write_output(const unsigned char *bytes, size_t length) {
    if (length > RV2_MAX_INPUT) rv_error("compute output exceeds 262144 bytes");
    rv_ignore_sigpipe();
    if (fwrite(bytes, 1, length, stdout) != length || fflush(stdout) != 0) {
        rv_system_error("cannot write compute output");
    }
}

void rv2_return_float(double value) {
    if (!isfinite(value)) rv_error("nonfinite Float output");
    rv2_finish_input();
    /* 17 significant digits roundtrip binary64; C starts in the C locale. */
    char output[64];
    int length = snprintf(output, sizeof(output), "%.16e\n", value);
    if (length < 0 || (size_t)length >= sizeof(output)) rv_error("cannot format Float output");
    rv2_write_output((const unsigned char *)output, (size_t)length);
}

void rv2_return(void *array) {
    rv_value *value = rv_require(array, RV_INT_ARRAY);
    rv2_finish_input();
    size_t count = value->length / sizeof(int64_t);
    const int64_t *items = (const int64_t *)(const void *)value->bytes;
    /* 20 digits/sign, comma and terminator per integer, plus brackets/newline. */
    size_t capacity = count * 22 + 4;
    rv_value *output = rv_allocate(capacity, RV_TEXT);
    size_t length = 0;
    output->bytes[length++] = '[';
    for (size_t index = 0; index < count; ++index) {
        if (index != 0) output->bytes[length++] = ',';
        int written = snprintf((char *)output->bytes + length, capacity - length, "%" PRId64, items[index]);
        if (written < 0 || (size_t)written >= capacity - length) rv_error("cannot format signed integer array");
        length += (size_t)written;
    }
    output->bytes[length++] = ']';
    output->bytes[length++] = '\n';
    rv2_write_output(output->bytes, length);
}

static rv2_list_node *rv2_node(void *list) {
    rv_value *value = rv_require(list, RV_INT_LIST);
    return (rv2_list_node *)(void *)value->bytes;
}

int64_t rv2_list_len(void *list) {
    return list == NULL ? 0 : (int64_t)rv2_node(list)->length;
}

/* Construct only after charging the operation's quota. A list read charges all
   nodes before token parsing, whereas cons charges its one newly created node. */
static void *rv2_new_node(int64_t item, void *tail, uint64_t length) {
    rv_value *value = rv_allocate(sizeof(rv2_list_node), RV_INT_LIST);
    rv2_list_node *node = (rv2_list_node *)(void *)value->bytes;
    node->value = item;
    node->tail = tail;
    node->length = length;
    return value;
}

void *rv2_list_cons(int64_t item, void *tail) {
    int64_t length = rv2_list_len(tail);
    if (length >= (int64_t)RV2_MAX_ITEMS) rv_error("integer list length exceeds 10000");
    rv2_charge(24);
    return rv2_new_node(item, tail, (uint64_t)length + 1);
}

int64_t rv2_list_value(void *list) {
    if (list == NULL) rv_error("cannot read value from nil list");
    return rv2_node(list)->value;
}

void *rv2_list_next(void *list) {
    if (list == NULL) rv_error("cannot read next from nil list");
    return rv2_node(list)->tail;
}

void *rv2_read_list(void) {
    int64_t count = rv2_read_int();
    if (count < 0) rv_error("integer list length must be nonnegative");
    if (count > (int64_t)RV2_MAX_ITEMS) rv_error("integer list length exceeds 10000");
    rv2_charge((size_t)count * 24);
    /* Bounded temporary input storage preserves stream order without exposing
       mutation or array-backed list handles. The published value is a chain of
       immutable nodes, sharing the same representation as ordinary cons. */
    int64_t items[RV2_MAX_ITEMS];
    for (size_t index = 0; index < (size_t)count; ++index) items[index] = rv2_read_int();
    void *list = NULL;
    for (size_t index = (size_t)count; index != 0; --index) {
        list = rv2_new_node(items[index - 1], list, (uint64_t)((size_t)count - index + 1));
    }
    return list;
}

void rv2_return_list(void *list) {
    size_t count = (size_t)rv2_list_len(list);
    rv2_finish_input();
    size_t capacity = count * 22 + 4;
    rv_value *output = rv_allocate(capacity, RV_TEXT);
    size_t length = 0;
    output->bytes[length++] = '[';
    void *cursor = list;
    for (size_t index = 0; index < count; ++index) {
        rv2_list_node *node = rv2_node(cursor);
        if (index != 0) output->bytes[length++] = ',';
        int written = snprintf((char *)output->bytes + length, capacity - length, "%" PRId64, node->value);
        if (written < 0 || (size_t)written >= capacity - length) rv_error("cannot format signed integer list");
        length += (size_t)written;
        cursor = node->tail;
    }
    output->bytes[length++] = ']';
    output->bytes[length++] = '\n';
    rv2_write_output(output->bytes, length);
}

void rv_add_route(const unsigned char *path, uint64_t length, uint32_t status, void *json) {
    if (rv_route_count == RV_MAX_ROUTES) rv_error("too many HTTP routes");
    if (path == NULL || length == 0 || length > RV_MAX_PATH || path[0] != '/') {
        rv_error("invalid HTTP route path");
    }
    for (size_t i = 0; i < (size_t)length; ++i) {
        if (path[i] <= 0x20 || path[i] >= 0x7f || path[i] == '?' || path[i] == '#') {
            rv_error("invalid HTTP route path");
        }
    }
    if (status < 200 || status > 299 || status == 204 || status == 205) {
        rv_error("invalid HTTP route status");
    }
    for (size_t i = 0; i < rv_route_count; ++i) {
        if (rv_routes[i].path->length == (size_t)length &&
            memcmp(rv_routes[i].path->bytes, path, (size_t)length) == 0) {
            rv_error("duplicate HTTP route path");
        }
    }
    rv_value *body = rv_require(json, RV_JSON);
    (void)rv_checked_size(body->length, "computed Json");
    rv_routes[rv_route_count++] = (rv_route){rv_text(path, length), body, status};
}

static void rv_stop(int signal_number) {
    (void)signal_number;
    rv_stopping = 1;
}

static int64_t rv_now_ms(void) {
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) rv_system_error("cannot read monotonic clock");
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

/* Poll in short intervals so a signal promptly cancels incomplete reads/writes. */
static int rv_wait(int descriptor, short events, int64_t deadline) {
    while (!rv_stopping) {
        int64_t remaining = deadline - rv_now_ms();
        if (remaining <= 0) return 0;
        int timeout = remaining > 25 ? 25 : (int)remaining;
        struct pollfd descriptor_state = {descriptor, events, 0};
        int result = poll(&descriptor_state, 1, timeout);
        if (result > 0) return 1;
        if (result < 0 && errno != EINTR) return -1;
    }
    return -1;
}

static int rv_send(int descriptor, const unsigned char *bytes, size_t length, int64_t deadline) {
    size_t offset = 0;
    while (offset < length && !rv_stopping) {
        if (rv_wait(descriptor, POLLOUT, deadline) <= 0) return 0;
        ssize_t sent = send(descriptor, bytes + offset, length - offset, 0);
        if (sent > 0) offset += (size_t)sent;
        else if (sent == 0 || (errno != EINTR && errno != EAGAIN && errno != EWOULDBLOCK)) return 0;
    }
    return offset == length;
}

static const char *rv_reason(uint32_t status) {
    switch (status) {
        case 200: return "OK";
        case 201: return "Created";
        case 202: return "Accepted";
        case 400: return "Bad Request";
        case 404: return "Not Found";
        case 405: return "Method Not Allowed";
        case 408: return "Request Timeout";
        case 413: return "Content Too Large";
        case 417: return "Expectation Failed";
        case 431: return "Request Header Fields Too Large";
        case 501: return "Not Implemented";
        default: return "Response";
    }
}

static void rv_response(int descriptor, uint32_t status, const unsigned char *body, size_t length) {
    char header[320];
    int header_length = snprintf(header, sizeof(header),
        "HTTP/1.1 %" PRIu32 " %s\r\nContent-Type: application/json; charset=utf-8\r\n"
        "Content-Length: %zu\r\nCache-Control: no-store\r\nConnection: close\r\n%s\r\n", status, rv_reason(status), length,
        status == 405 ? "Allow: GET\r\n" : "");
    if (header_length < 0 || (size_t)header_length >= sizeof(header)) rv_error("cannot format HTTP response");
    int64_t deadline = rv_now_ms() + 250;
    /* Keep small error responses in one write. Closing rejected input may
       reset TCP when unread bytes remain, so avoid splitting header and body. */
    if ((size_t)header_length + length <= 512) {
        unsigned char response[512];
        memcpy(response, header, (size_t)header_length);
        memcpy(response + (size_t)header_length, body, length);
        (void)rv_send(descriptor, response, (size_t)header_length + length, deadline);
        return;
    }
    if (rv_send(descriptor, (const unsigned char *)header, (size_t)header_length, deadline)) {
        (void)rv_send(descriptor, body, length, deadline);
    }
}

typedef struct {
    char method[64];
    char path[RV_MAX_PATH + 1];
} rv_request;

static int rv_token(unsigned char byte) {
    if ((byte >= 'a' && byte <= 'z') || (byte >= 'A' && byte <= 'Z') ||
        (byte >= '0' && byte <= '9')) return 1;
    return byte != 0 && strchr("!#$%&'*+-.^_`|~", byte) != NULL;
}

static int rv_header_equal(const char *bytes, size_t length, const char *name) {
    return strlen(name) == length && strncasecmp(bytes, name, length) == 0;
}

static uint32_t rv_parse_headers(char *bytes, size_t header_end, rv_request *request,
                                 size_t *body_length) {
    char *line_end = strstr(bytes, "\r\n");
    if (line_end == NULL) return 400;
    char *space = memchr(bytes, ' ', (size_t)(line_end - bytes));
    if (space == NULL || space == bytes || (size_t)(space - bytes) >= sizeof(request->method)) return 400;
    for (char *p = bytes; p < space; ++p) if (!rv_token((unsigned char)*p)) return 400;
    size_t method_length = (size_t)(space - bytes);
    memcpy(request->method, bytes, method_length);
    request->method[method_length] = '\0';
    char *target = space + 1;
    space = memchr(target, ' ', (size_t)(line_end - target));
    if (space == NULL || space == target || target[0] != '/') return 400;
    size_t target_length = (size_t)(space - target);
    if (target_length > RV_MAX_PATH) return 400;
    for (size_t i = 0; i < target_length; ++i) {
        if ((unsigned char)target[i] <= 0x20 || (unsigned char)target[i] >= 0x7f || target[i] == '#') return 400;
    }
    if (line_end - space != 9 ||
        (memcmp(space + 1, "HTTP/1.0", 8) != 0 && memcmp(space + 1, "HTTP/1.1", 8) != 0)) return 400;
    char *query = memchr(target, '?', target_length);
    size_t path_length = query == NULL ? target_length : (size_t)(query - target);
    memcpy(request->path, target, path_length);
    request->path[path_length] = '\0';
    int seen_length = 0;
    int seen_host = 0;
    size_t header_count = 0;
    char *line = line_end + 2;
    while (line < bytes + header_end - 2) {
        if (++header_count > 64) return 431;
        line_end = strstr(line, "\r\n");
        if (line_end == NULL) return 400;
        char *colon = memchr(line, ':', (size_t)(line_end - line));
        if (colon == NULL || colon == line) return 400;
        for (char *p = line; p < colon; ++p) if (!rv_token((unsigned char)*p)) return 400;
        char *value = colon + 1;
        char *value_end = line_end;
        while (value < value_end && (*value == ' ' || *value == '\t')) ++value;
        while (value_end > value && (value_end[-1] == ' ' || value_end[-1] == '\t')) --value_end;
        for (char *p = value; p < value_end; ++p) {
            if (((unsigned char)*p < 0x20 && *p != '\t') || (unsigned char)*p == 0x7f) return 400;
        }
        size_t name_length = (size_t)(colon - line);
        if (rv_header_equal(line, name_length, "transfer-encoding")) return 501;
        if (rv_header_equal(line, name_length, "expect")) return 417;
        if (rv_header_equal(line, name_length, "host")) {
            if (seen_host) return 400;
            seen_host = 1;
        }
        if (rv_header_equal(line, name_length, "content-length")) {
            if (seen_length || value == value_end) return 400;
            seen_length = 1;
            size_t length = 0;
            for (char *p = value; p < value_end; ++p) {
                if (*p < '0' || *p > '9') return 400;
                size_t digit = (size_t)(*p - '0');
                if (length > RV_MAX_BODY / 10 || length * 10 + digit > RV_MAX_BODY) return 413;
                length = length * 10 + digit;
            }
            *body_length = length;
        }
        line = line_end + 2;
    }
    return 0;
}

static uint32_t rv_read_request(int descriptor, rv_request *request) {
    char bytes[RV_MAX_HEADERS + 1];
    size_t used = 0;
    size_t header_end = 0;
    int64_t deadline = rv_now_ms() + 2000;
    while (!rv_stopping && header_end == 0) {
        if (used == RV_MAX_HEADERS) return 431;
        int ready = rv_wait(descriptor, POLLIN, deadline);
        if (ready <= 0) return ready == 0 ? 408 : 400;
        ssize_t received = recv(descriptor, bytes + used, RV_MAX_HEADERS - used, 0);
        if (received == 0) return 400;
        if (received < 0) {
            if (errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK) continue;
            return 400;
        }
        used += (size_t)received;
        bytes[used] = '\0';
        for (size_t i = 3; i < used; ++i) {
            if (memcmp(bytes + i - 3, "\r\n\r\n", 4) == 0) { header_end = i + 1; break; }
        }
    }
    if (rv_stopping) return 400;
    if (memchr(bytes, '\0', header_end) != NULL) return 400;
    bytes[header_end - 1] = '\0';
    size_t body_length = 0;
    uint32_t error = rv_parse_headers(bytes, header_end, request, &body_length);
    if (error != 0) return error;
    size_t received_body = used - header_end;
    unsigned char discard[4096];
    while (received_body < body_length && !rv_stopping) {
        int ready = rv_wait(descriptor, POLLIN, deadline);
        if (ready <= 0) return ready == 0 ? 408 : 400;
        size_t wanted = body_length - received_body;
        if (wanted > sizeof(discard)) wanted = sizeof(discard);
        ssize_t received = recv(descriptor, discard, wanted, 0);
        if (received == 0) return 400;
        if (received < 0) {
            if (errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK) continue;
            return 400;
        }
        received_body += (size_t)received;
    }
    return rv_stopping ? 400 : 0;
}

static int rv_nonblocking(int descriptor) {
    int flags = fcntl(descriptor, F_GETFL, 0);
    return flags < 0 ? -1 : fcntl(descriptor, F_SETFL, flags | O_NONBLOCK);
}

int32_t rv_serve(uint32_t port) {
    if (port > 65535) rv_error("invalid HTTP port");
    rv_ignore_sigpipe();
    struct sigaction action;
    struct sigaction old_interrupt;
    struct sigaction old_terminate;
    memset(&action, 0, sizeof(action));
    action.sa_handler = rv_stop;
    (void)sigemptyset(&action.sa_mask);
    rv_stopping = 0;
    if (sigaction(SIGINT, &action, &old_interrupt) != 0 ||
        sigaction(SIGTERM, &action, &old_terminate) != 0) rv_system_error("cannot install stop handlers");
    int listener = socket(AF_INET, SOCK_STREAM, 0);
    if (listener < 0) rv_system_error("cannot create HTTP listener");
    int reuse = 1;
    if (setsockopt(listener, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse)) != 0 ||
        rv_nonblocking(listener) != 0) rv_system_error("cannot configure HTTP listener");
    struct sockaddr_in address;
    memset(&address, 0, sizeof(address));
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(UINT32_C(0x7f000001));
    address.sin_port = htons((uint16_t)port);
    if (bind(listener, (const struct sockaddr *)(const void *)&address, sizeof(address)) != 0) {
        rv_system_error("cannot bind HTTP listener on 127.0.0.1");
    }
    if (listen(listener, 16) != 0) rv_system_error("cannot listen for HTTP requests");
    socklen_t address_length = sizeof(address);
    if (getsockname(listener, (struct sockaddr *)(void *)&address, &address_length) != 0) {
        rv_system_error("cannot read HTTP listener address");
    }
    if (printf("Rivet program server: http://127.0.0.1:%u\n", (unsigned)ntohs(address.sin_port)) < 0 ||
        fflush(stdout) != 0) rv_system_error("cannot report HTTP listener address");
    const char *control = getenv("RIVET_CONTROL");
    int log_requests = control != NULL && strcmp(control, "1") == 0;
    while (!rv_stopping) {
        int ready = rv_wait(listener, POLLIN, rv_now_ms() + 1000);
        if (ready < 0 && !rv_stopping) rv_system_error("cannot poll HTTP listener");
        if (ready <= 0) continue;
        int client = accept(listener, NULL, NULL);
        if (client < 0) {
            if (errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK) continue;
            rv_system_error("cannot accept HTTP request");
        }
        if (rv_nonblocking(client) != 0) { (void)close(client); continue; }
        rv_request request;
        uint32_t status = rv_read_request(client, &request);
        if (!rv_stopping) {
            static const unsigned char missing[] = "{\"error\":\"not found\"}";
            static const unsigned char method[] = "{\"error\":\"method not allowed\"}";
            static const unsigned char malformed[] = "{\"error\":\"invalid HTTP request\"}";
            const unsigned char *body = malformed;
            size_t body_length = sizeof(malformed) - 1;
            if (status == 0 && strcmp(request.method, "GET") != 0) {
                status = 405; body = method; body_length = sizeof(method) - 1;
            } else if (status == 0) {
                status = 404; body = missing; body_length = sizeof(missing) - 1;
                size_t path_length = strlen(request.path);
                for (size_t i = 0; i < rv_route_count; ++i) {
                    if (rv_routes[i].path->length == path_length &&
                        memcmp(rv_routes[i].path->bytes, request.path, path_length) == 0) {
                        status = rv_routes[i].status;
                        body = rv_routes[i].body->bytes;
                        body_length = rv_routes[i].body->length;
                        break;
                    }
                }
            }
            rv_response(client, status, body, body_length);
            if (log_requests && (fputs("RIVET_REQUEST\n", stderr) == EOF || fflush(stderr) != 0)) {
                rv_system_error("cannot report HTTP request");
            }
        }
        (void)close(client);
    }
    (void)close(listener);
    (void)sigaction(SIGINT, &old_interrupt, NULL);
    (void)sigaction(SIGTERM, &old_terminate, NULL);
    return 0;
}

void rv_cleanup(void) {
    while (rv_allocations != NULL) {
        rv_value *value = rv_allocations;
        rv_allocations = value->next;
        free(value);
    }
    rv_route_count = 0;
    free(rv2_input);
    rv2_input = NULL;
    rv2_input_length = 0;
    rv2_input_cursor = 0;
    rv2_allocation_charge = 0;
    rv2_steps = 0;
}
