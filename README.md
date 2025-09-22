# chip8-web-toolchain
## About
Interpreter, debugger, disassembler, and compiler for the CHIP-8. Written in Rust, accessible from a web interface via WebAssembly. The compiler compiles a high-level C-like language to the CHIP-8 bytecode, and has a small standard library - an example program that demonstrates most features of the language is loaded by default in the web interface's code editor.

View the deployment at [https://retro-git.github.io/chip8-web-toolchain/.](https://retro-meister.github.io/chip8-web-toolchain/)

## Build

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
