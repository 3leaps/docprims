# docprims (TypeScript bindings)

TypeScript/Node.js bindings for docprims using a Node-API (N-API) native addon (napi-rs).

Requires Node.js 22 or newer.

## Platform support (v0.1.x)

Supported:

- macOS: arm64
- Linux: glibc and musl (Alpine)
- Windows: x64 (msvc)

## Local development

The native addon is built from Rust:

```bash
npm install
npm run build
npm run build:native
npm run test:ci
```

## API (minimal)

- `extractFile(path, options?)` -> parsed v0 extract object
- `extractBytes(sourceUri, data, options?)` -> parsed v0 extract object

The `*Json` variants return the raw JSON string.
