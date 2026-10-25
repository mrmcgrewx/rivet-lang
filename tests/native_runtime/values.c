/* White-box support-library checks; generated programs use only runtime.h. */
#include "../../native/runtime.c"
#include <assert.h>

static void expect(void *pointer, rv_kind kind, const unsigned char *bytes, size_t length) {
    rv_value *value = pointer;
    assert(value->kind == kind);
    assert(value->length == length);
    assert(length == 0 || memcmp(value->bytes, bytes, length) == 0);
}

static void check_values(void) {
    const unsigned char raw[] = {'a', 0, '\n', '"', '\\', '\b', '\f', '\r', '\t', 1,
                                 0xf0, 0x9f, 0x99, 0x82};
    const unsigned char encoded[] = "\"a\\u0000\\n\\\"\\\\\\b\\f\\r\\t\\u0001\xf0\x9f\x99\x82\"";
    void *text = rv_text(raw, sizeof(raw));
    expect(text, RV_TEXT, raw, sizeof(raw));
    expect(rv_json_text(text), RV_JSON, encoded, sizeof(encoded) - 1);
    expect(text, RV_TEXT, raw, sizeof(raw));
    expect(rv_text(NULL, 0), RV_TEXT, NULL, 0);
    expect(rv_concat(NULL, 0), RV_TEXT, NULL, 0);
    void *pieces[] = {rv_text(NULL, 0), text, rv_text((const unsigned char *)"z", 1)};
    unsigned char joined[sizeof(raw) + 1];
    memcpy(joined, raw, sizeof(raw));
    joined[sizeof(raw)] = 'z';
    expect(rv_concat(pieces, 3), RV_TEXT, joined, sizeof(joined));
    expect(rv_i64_text(INT64_MIN), RV_TEXT, (const unsigned char *)"-9223372036854775808", 20);
    expect(rv_i64_text(INT64_MAX), RV_TEXT, (const unsigned char *)"9223372036854775807", 19);
    expect(rv_json_i64(INT64_MIN), RV_JSON, (const unsigned char *)"-9223372036854775808", 20);
    expect(rv_json_i64(INT64_MAX), RV_JSON, (const unsigned char *)"9223372036854775807", 19);
    expect(rv_bool_text(0), RV_TEXT, (const unsigned char *)"false", 5);
    expect(rv_bool_text(1), RV_TEXT, (const unsigned char *)"true", 4);
    expect(rv_json_bool(0), RV_JSON, (const unsigned char *)"false", 5);
    expect(rv_json_bool(1), RV_JSON, (const unsigned char *)"true", 4);
    expect(rv_object(NULL, NULL, NULL, 0), RV_JSON, (const unsigned char *)"{}", 2);
    const unsigned char *keys[] = {(const unsigned char *)"z", (const unsigned char *)"a",
                                  (const unsigned char *)"a", (const unsigned char *)"\0"};
    uint64_t lengths[] = {1, 1, 1, 1};
    void *items[] = {rv_json_bool(0), rv_json_i64(1), rv_json_i64(2), rv_json_bool(1)};
    void *object = rv_object(keys, lengths, items, 4);
    const unsigned char expected[] = "{\"\\u0000\":true,\"a\":2,\"z\":false}";
    expect(object, RV_JSON, expected, sizeof(expected) - 1);
    const unsigned char *nested_keys[] = {(const unsigned char *)"nested"};
    uint64_t nested_lengths[] = {6};
    void *nested_items[] = {object};
    const unsigned char nested_expected[] = "{\"nested\":{\"\\u0000\":true,\"a\":2,\"z\":false}}";
    expect(rv_object(nested_keys, nested_lengths, nested_items, 1), RV_JSON,
           nested_expected, sizeof(nested_expected) - 1);

    unsigned char large[65536];
    memset(large, 0, sizeof(large));
    rv_value *escaped = rv_json_text(rv_text(large, sizeof(large)));
    assert(escaped->length == 6 * sizeof(large) + 2);
    memset(large, 'a', sizeof(large));
    const unsigned char *large_keys[] = {(const unsigned char *)"x"};
    uint64_t large_lengths[] = {1};
    void *large_items[] = {rv_json_text(rv_text(large, 65528))};
    rv_value *large_object = rv_object(large_keys, large_lengths, large_items, 1);
    assert(large_object->length == 65536);
    assert(memcmp(large_object->bytes, "{\"x\":\"", 6) == 0);
    assert(memcmp(large_object->bytes + 65534, "\"}", 2) == 0);
    rv_cleanup();
    rv_cleanup();
    assert(rv_allocations == NULL && rv_route_count == 0);
    expect(rv_text((const unsigned char *)"again", 5), RV_TEXT,
           (const unsigned char *)"again", 5);
    rv_cleanup();
}

int main(int argc, char **argv) {
    if (argc == 1) { check_values(); return 0; }
    unsigned char large[65536];
    memset(large, 'x', sizeof(large));
    if (strcmp(argv[1], "concat") == 0) {
        void *parts[] = {rv_text(large, sizeof(large)), rv_text(large, 1)};
        (void)rv_concat(parts, 2);
    } else if (strcmp(argv[1], "output") == 0) {
        rv_print(rv_text(large, sizeof(large)));
    } else if (strcmp(argv[1], "output-ok") == 0) {
        rv_print(rv_text(large, sizeof(large) - 1));
    } else if (strcmp(argv[1], "json") == 0) {
        const unsigned char *keys[] = {(const unsigned char *)"x"};
        uint64_t lengths[] = {1};
        void *items[] = {rv_json_text(rv_text(large, 65529))};
        (void)rv_object(keys, lengths, items, 1);
    } else if (strcmp(argv[1], "escaped-json") == 0) {
        memset(large, 0, sizeof(large));
        const unsigned char *keys[] = {(const unsigned char *)"x"};
        uint64_t lengths[] = {1};
        void *items[] = {rv_json_text(rv_text(large, sizeof(large)))};
        (void)rv_object(keys, lengths, items, 1);
    } else if (strcmp(argv[1], "nul-output") == 0) {
        const unsigned char raw[] = {'a', 0, 'b'};
        rv_print(rv_text(raw, sizeof(raw)));
    } else {
        return 2;
    }
    rv_cleanup();
    return 0;
}
