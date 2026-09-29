# ting-reader-plugin-contract

The versioned public contract for Ting Reader plugins.

This crate contains the data types and ABI declarations shared by the Ting
Reader core, plugin SDKs, and third-party plugins:

- plugin manifests, permissions, capabilities, and version floors;
- format declarations, format calls, metadata extraction, decryption, and
  staging writeback DTOs;
- scraper request and result envelopes;
- resource handles and host resource operations;
- transport-independent call results and plugin error codes;
- the Native ABI v2 Rust declarations and matching C header.

The contract crate deliberately does not depend on the Ting Reader backend.
It does not contain database, HTTP server, runtime, filesystem, WASM, JavaScript
or plugin business logic. Those responsibilities remain in the core or in the
runtime-specific SDKs.

## Using it

The Ting Reader backend pins a released tag:

```toml
ting-plugin-contract = { git = "https://github.com/dqsq2e2/ting-reader-plugin-contract.git", tag = "v2.0.0" }
```

Plugin SDKs and plugins use the same dependency, so the host and extensions
compile against one exact contract revision. A contract release is immutable:
breaking changes require a new major version and a new plugin core floor.

## Format extensions

The core only owns the format operation contract and dispatch path. A special
format implementation, including metadata-only formats such as XM, belongs in
an independent extension. Such an extension declares its supported extensions
and operations in its plugin manifest and implements the corresponding calls
through an SDK; its parser and format-specific metadata stay outside the core.

## Versioning

`v2.0.0` is the first externally published contract for the standardized
plugin foundation. The `MIN_PLUGIN_CORE_VERSION` constant is the minimum core
release that accepts this contract.

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
