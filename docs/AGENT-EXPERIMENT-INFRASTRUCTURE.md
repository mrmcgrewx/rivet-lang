# Experiment infrastructure notes

The nine measured authoring trials used the frozen `7461e96` harness, packets and corpora. All nine passed their first development submission. No submitted solver or frozen harness was changed afterward.

## Local AddressSanitizer startup failure

The post-authoring C address/undefined sanitizer executables hung before producing output on this macOS 26.5.2 / Apple Clang 17 host. A separate empty `int main(void) { return 0; }` executable built with `-fsanitize=address,undefined` reproduced the hang. It remained active after eight seconds; a submitted solution also timed out after twenty seconds. These are infrastructure probes, not additional worker submissions.

A one-second sample of the empty process showed initialization recursively entering ASan through allocation during dyld shared-cache enumeration:

```text
__asan::AsanInitInternal
  __asan::InitializeShadowMemory
    __sanitizer::MemoryRangeIsAvailable
      __sanitizer::MemoryMappingLayout::Next
        __sanitizer::get_dyld_hdr
          dyld_shared_cache_iterate_text_swift
            _Block_copy
              __sanitizer_mz_malloc
                __asan::AsanInitFromRtl
                  __sanitizer::StaticSpinMutex::LockSlow
```

The initial post-freeze evaluation was stopped during the first C sanitizer suite. No incomplete evaluation was used as an authoring or correctness result. Its ordinary native tests were rerun with the exact same frozen sources and cases. Local supplementary C checks use working UBSan; `agent-experiment-safety.yml` runs all frozen C holdouts under ASan and UBSan on Ubuntu 24.04. Linux's explicit `-lm` link is infrastructure support only. CI sanitizer results are correctness evidence, never performance samples. Performance remains on the original Apple Silicon host with the originally specified unsanitized compiler settings.

## Additional review ownership

After measured workers and timing finish, an independent, unmeasured reviewer may inspect the frozen sources, numeric usage, scoped command audit and reports. It owns only `docs/AGENT-EXPERIMENT-REVIEW.md`, must not change any solver or frozen harness, and must persist findings before ending. The coordinator owns integration, fixes to non-frozen reporting, clean reproduction, final documentation and Git.
