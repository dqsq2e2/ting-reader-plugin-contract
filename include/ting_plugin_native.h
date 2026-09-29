/* Ting Reader Native plugin ABI revision 2.
 * This header mirrors src/native_abi.rs. Only `ting_plugin_abi_v2` is exported.
 * Pointers passed to supports/invoke are borrowed for that call only.
 * Plugin output is allocated by Host through allocator.allocate, and the Host
 * owns and reclaims it after invoke returns. Never free it from the plugin.
 * Native libraries run in the server process; a crash can terminate it.
 */
#ifndef TING_PLUGIN_NATIVE_H
#define TING_PLUGIN_NATIVE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define TING_NATIVE_ABI_REVISION 2u
#define TING_NATIVE_MAX_CONTROL_BYTES (1024u * 1024u)

typedef struct TingNativeAbiHeader {
    uint32_t abi_revision;
    uint32_t struct_size;
} TingNativeAbiHeader;

typedef struct TingNativeHostAllocator {
    void *user_data;
    uint8_t *(*allocate)(void *user_data, size_t size);
} TingNativeHostAllocator;

typedef struct TingNativeTargetInfo {
    uint32_t pointer_width;
    uint32_t byte_order; /* 1=little endian, 2=big endian */
} TingNativeTargetInfo;

typedef enum TingNativeStatus {
    TING_NATIVE_OK = 0,
    TING_NATIVE_INVALID_INPUT = -1,
    TING_NATIVE_NO_SCOPE = -2,
    TING_NATIVE_NOT_FOUND = -3,
    TING_NATIVE_PERMISSION_DENIED = -4,
    TING_NATIVE_RESOURCE_LIMIT = -5,
    TING_NATIVE_CANCELLED = -6,
    TING_NATIVE_INTERNAL_ERROR = -7,
    TING_NATIVE_UNSUPPORTED_OPERATION = -8
} TingNativeStatus;

/* This table and its user_data are valid only during invoke. The Host owns
 * binary media and reserves no plugin-visible filesystem paths or addresses.
 * A positive result length is returned through *actual; status is zero.
 */
typedef struct TingNativeHostApiV2 {
    TingNativeAbiHeader header;
    TingNativeTargetInfo target;
    void *user_data;
    int32_t (*invoke)(
        void *user_data, const uint8_t *method, size_t method_len,
        const uint8_t *input_json, size_t input_len,
        uint8_t *output_json, size_t output_capacity, size_t *actual
    );
    int32_t (*read_at)(
        void *user_data, const uint8_t *resource_id, size_t id_len,
        uint64_t offset, uint8_t *output, size_t output_capacity,
        size_t *actual, bool *eof
    );
    int32_t (*write_at)(
        void *user_data, const uint8_t *resource_id, size_t id_len,
        uint64_t offset, const uint8_t *input, size_t input_len, size_t *written
    );
    int32_t (*chunk_create)(
        void *user_data, const uint8_t *input, size_t input_len,
        uint8_t *id_output, size_t id_capacity, size_t *id_len
    );
    int32_t (*chunk_copy)(
        void *user_data, const uint8_t *chunk_id, size_t id_len,
        uint8_t *output, size_t capacity, size_t *actual
    );
} TingNativeHostApiV2;

typedef struct TingNativeCallOutput {
    uint8_t *data;
    size_t len;
} TingNativeCallOutput;

typedef struct TingNativePluginAbiV2 {
    TingNativeAbiHeader header;
    TingNativeTargetInfo target;
    void *(*create)(void);
    void (*destroy)(void *instance);
    bool (*supports)(void *instance, const uint8_t *operation, size_t operation_len);
    int32_t (*invoke)(
        void *instance,
        const uint8_t *operation,
        size_t operation_len,
        const uint8_t *input_json,
        size_t input_len,
        const TingNativeHostApiV2 *host,
        TingNativeHostAllocator allocator,
        TingNativeCallOutput *output
    );
} TingNativePluginAbiV2;

/* A plugin implements this one symbol, returning a stable, static table.
 * The Host checks header.abi_revision and header.struct_size before reading
 * the remaining function pointers. The table and functions must remain valid
 * until every call returns and the library is unloaded.
 */
const TingNativeAbiHeader *ting_plugin_abi_v2(void);

#ifdef __cplusplus
}
#endif
#endif
