# Optional local semantic inference

The `semantic-knowledge` Cargo feature is off by default. Building it does not
fetch a model or an ONNX Runtime library. The inference loader accepts local
model files whose hashes match `crates/cli/src/model_manifest.json` and requires
a separate runtime manifest under the supplied model directory:

```text
<model-dir>/
  config.json
  tokenizer.json
  tokenizer_config.json
  special_tokens_map.json
  onnx/model.onnx
  runtime/
    runtime-manifest.json
    <runtime library files>
```

`runtime-manifest.json` identifies the local shared library, its SHA-256,
companion libraries, target platform, and build attestation. It must use ONNX
Runtime 1.24.4, API 24, and upstream source commit
`2d924974ef147392ced8409d36bd6d2e7fcc8a74`. The accepted platform strings are
`windows-x86_64`, `linux-x86_64`, and `macos-x86_64`. `main_library.path` must
be a filename in `runtime/`; companion paths must remain relative to that
directory. The loader rejects traversal and symlinks and hashes every listed
runtime file before loading it.

Example shape (replace library names and hash values with those from the local
build):

```json
{
  "version": "1.24.4",
  "api": 24,
  "source_repository": "https://github.com/microsoft/onnxruntime",
  "source_commit": "2d924974ef147392ced8409d36bd6d2e7fcc8a74",
  "platform": "windows-x86_64",
  "cpu_only": true,
  "telemetry_disabled": true,
  "build_options": [
    "onnxruntime_USE_TELEMETRY=OFF",
    "onnxruntime_BUILD_SHARED_LIB=ON"
  ],
  "main_library": {
    "path": "onnxruntime.dll",
    "sha256": "<64 lowercase hexadecimal digits>"
  },
  "companion_libraries": [
    {
      "path": "onnxruntime_providers_shared.dll",
      "sha256": "<64 lowercase hexadecimal digits>"
    }
  ]
}
```

The manifest is an operator-reviewed local assertion. Its hash detects a file
that differs from the reviewed manifest, but the unsigned manifest and binary
do not prove build provenance or prove that telemetry was disabled. Build
ONNX Runtime from the pinned source with telemetry disabled, inspect the
resulting build configuration, then record the exact library hashes. The
loader rejects the known telemetry-enabled Microsoft Windows 1.24.4 DLL even
if its local manifest claims otherwise. No public archive currently includes
this optional runtime.

CI checks optional-feature compilation and asset-validation tests on Windows,
Linux, and macOS Intel. Those checks do not provision runtime/model assets and
do not establish successful native inference on those platforms. Native
inference requires the local model and a reviewed local runtime manifest.
