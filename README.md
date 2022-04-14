# c8-web-toolchain

## Building

Compile the Rust backend code with `wasm-pack`:

```bash
wasm-pack build
```

Install `npm` dependencies:

```bash
npm install
```

Run development server:

```bash
npm run start
```

Use webpack to output distribution files to `dist` folder:

```bash
npm run build
```

Run Rust unit tests:

```bash
cargo test
```
