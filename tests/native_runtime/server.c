#include "../../native/runtime.h"
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
    const unsigned char *keys[] = {(const unsigned char *)"z", (const unsigned char *)"min",
                                  (const unsigned char *)"max", (const unsigned char *)"text"};
    uint64_t lengths[] = {1, 3, 3, 4};
    const unsigned char text[] = {'h', 'i', 0, '\n', 0xf0, 0x9f, 0x99, 0x82};
    void *values[] = {rv_json_bool(1), rv_json_i64(INT64_MIN), rv_json_i64(INT64_MAX),
                      rv_json_text(rv_text(text, sizeof(text)))};
    void *body = rv_object(keys, lengths, values, 4);
    rv_add_route((const unsigned char *)"/hello", 6, 201, body);
    unsigned char long_path[2048];
    memset(long_path, 'x', sizeof(long_path));
    long_path[0] = '/';
    rv_add_route(long_path, sizeof(long_path), 200, body);
    unsigned char large[65528];
    memset(large, 'x', sizeof(large));
    const unsigned char *large_keys[] = {(const unsigned char *)"x"};
    uint64_t large_lengths[] = {1};
    void *large_values[] = {rv_json_text(rv_text(large, sizeof(large)))};
    rv_add_route((const unsigned char *)"/large", 6, 200,
                 rv_object(large_keys, large_lengths, large_values, 1));
    uint32_t port = argc > 1 ? (uint32_t)strtoul(argv[1], NULL, 10) : 0;
    int32_t status = rv_serve(port);
    rv_cleanup();
    return status;
}
