import { memory } from "../pkg/c8_web_toolchain_bg.wasm";
import { Lexer, Compiler, Assembler, echo_string } from "../pkg/c8_web_toolchain";

var num_disasm_rows = 21;

export class emulator_view {
    constructor(chip8, scale, editor, framebuffer_parent, register_parent, disasm_parent) {
        this.chip8 = chip8;
        this.editor = editor;
        this.scale = scale;
        this.ram_line_map = new Object();
        this.framebuffer_parent = framebuffer_parent;
        this.canvas = framebuffer_parent.appendChild(document.createElement("canvas"));
        this.canvas.style.border = '2px solid grey';
        this.ctx = this.canvas.getContext('2d');
        this.set_scale(scale);
        this.paused = false;

        this.key_mappings = new Map([
            [0x1, "Digit1"], [0x2, "Digit2"], [0x3, "Digit3"], [0xC, "Digit4"],
            [0x4, "KeyQ"], [0x5, "KeyW"], [0x6, "KeyE"], [0xD, "KeyR"],
            [0x7, "KeyA"], [0x8, "KeyS"], [0x9, "KeyD"], [0xE, "KeyF"],
            [0xA, "KeyZ"], [0x0, "KeyX"], [0xB, "KeyC"], [0xF, "KeyV"],
        ]);

        let pause_button = document.createElement("button"); pause_button.innerHTML = "Pause/Play";
        pause_button.onclick = this.onClickPauseButton.bind(this); pause_button.setAttribute("class", "btn btn-secondary");
        register_parent.appendChild(pause_button);
        let step_button = document.createElement("button"); step_button.innerHTML = "Step Forward";
        step_button.onclick = this.onClickStepButton.bind(this); step_button.setAttribute("class", "btn btn-secondary")
        disasm_parent.appendChild(step_button);

        this.register_list = register_parent.appendChild(document.createElement("ul"));
        this.register_list.setAttribute("class", "list-group list-group-mine");

        for (let i = 0; i < 21; i++) {
            let li = this.register_list.appendChild(document.createElement("li"));
            li.setAttribute("class", "list-group-item py-0 list-group-item-secondary");
        }

        document.getElementById("save").addEventListener("click", this.onClickSaveState.bind(this));
        document.getElementById("load").addEventListener("click", this.onClickLoadState.bind(this));
        document.getElementById("sourceCodeSubmit").addEventListener("click", this.onClickSubmit.bind(this));

        this.disasm_list = disasm_parent.appendChild(document.createElement("ul"));
        this.disasm_list.setAttribute("class", "list-group list-group-mine");
        for (let i = 0; i < num_disasm_rows; i++) {
            let li = this.disasm_list.appendChild(document.createElement("li"));
            li.setAttribute("class", "list-group-item list-group-item-secondary py-0");
            li.setAttribute("id", `${i}`);
        }

        document.addEventListener("keydown", this.onKeyDown.bind(this));
        document.addEventListener("keyup", this.onKeyUp.bind(this));
    }

    onClickSaveState() {
        this.chip8.save_state();
    }

    onClickLoadState() {
        this.chip8.load_state();
    }

    onClickPauseButton() {
        this.paused = !this.paused;
        console.log(this.paused);
    }

    onClickStepButton() {
        if (!this.paused) this.paused = true;
        this.step();
    }


    onKeyDown(e) {
        for (const [key, value] of this.key_mappings.entries()) {
            if (e.code == value) {
                this.chip8.set_key(key, 1);
            }
        }
    }

    onKeyUp(e) {
        for (const [key, value] of this.key_mappings) {
            if (e.code == value) {
                this.chip8.set_key(key, 0);
            }
        }
    }

    onClickSubmit() {
        var editor = ace.edit("editor");
        let lexer = Lexer.new(editor.getValue());
        lexer.lex();

        let compiler = Compiler.new_from_lexer(lexer);
        compiler.compile();
        this.ram_line_map = compiler.ram_line_map_serialised();

        let assembler = Assembler.new_from_compiler(compiler);
        assembler.assemble();
        this.chip8.load_rom_from_assembler(assembler);

        this.chip8.disassemble();
        this.disasm_map = this.chip8.disasm_map_serialised();

        this.draw_framebuffer();
        this.draw_disasm();
        this.draw_registers();

        document.getElementById("lexerOutputTextarea").value = lexer.stringify_tokens();
        document.getElementById("compilerOutputTextarea").value = compiler.stringify_asm();
        document.getElementById("assemblerOutputTextarea").value = assembler.stringify_binary();
    }

    async load_rom_from_file(filename) {
        const response = await fetch(`roms/${filename}`);
        const buffer = await response.arrayBuffer();
        const array = new Uint8Array(buffer);
        this.chip8.load_rom_from_bytes(array);

        this.chip8.disassemble();
        this.disasm_map = this.chip8.disasm_map_serialised();
    }

    draw_framebuffer() {
        const framebuffer_ptr = this.chip8.framebuffer();
        const framebuffer = new Uint32Array(memory.buffer, framebuffer_ptr, this.chip8.video_width() * this.chip8.video_height());

        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        this.ctx.beginPath();

        for (let row = 0; row < this.chip8.video_height(); row++) {
            for (let col = 0; col < this.chip8.video_width(); col++) {
                const idx = row * this.chip8.video_width() + col;

                this.ctx.fillStyle = framebuffer[idx] === 0 ? "#000000" : "#FFFFFF";

                this.ctx.fillRect(col, row, 1, 1);
            }
        }

        this.ctx.stroke();
    }

    draw_registers() {
        const V_ptr = this.chip8.V();
        const V = new Uint8Array(memory.buffer, V_ptr, 16);

        let list = this.register_list.getElementsByTagName("li");

        list[0].innerHTML = "PC: " + this.chip8.pc().toString(16);
        list[1].innerHTML = "I: " + this.chip8.I().toString(16);
        list[2].innerHTML = "SP: " + this.chip8.sp().toString(16);
        list[3].innerHTML = "DT: " + this.chip8.delay_timer().toString(16);
        list[4].innerHTML = "ST: " + this.chip8.sound_timer().toString(16);

        for (let i = 0; i < V.length; i++) {
            list[i + 5].innerHTML = "V" + i.toString(16).toUpperCase() + ": " + V[i].toString(16).toUpperCase();
        }

    }

    draw_disasm() {
        let list = this.disasm_list.getElementsByTagName("li");

        for (let i = 0; i < num_disasm_rows; i++) {
            list[i].style.backgroundColor = "#272822"
            let pc = this.chip8.pc() + i * 2;
            list[i].innerHTML = "0x" + pc.toString(16).toUpperCase() + ": " + this.disasm_map[pc];
            if (pc.toString() in this.ram_line_map) {
                let end = this.editor.getSelectionRange().end.row, start = this.editor.getSelectionRange().start.row;
                for (var line = start; line <= end; line++) {
                    if (this.ram_line_map[pc] == line) {
                        list[i].style.backgroundColor = "#800000";
                    }
                }
            }
        }

    }

    step() {
        this.chip8.clock();

        this.draw_framebuffer();
        this.draw_registers();
        this.draw_disasm();
    }

    emulation_loop() {
        if (!this.paused) {
            this.step();
        }
        setTimeout(this.emulation_loop.bind(this), (1 / 240) * 1000);
    }

    set_scale(scale) {
        //this.scale = scale;
        this.canvas.width = this.chip8.video_width() * scale;
        this.canvas.height = this.chip8.video_height() * scale;
        this.ctx.scale(scale, scale);
        this.draw_framebuffer();
    }
}