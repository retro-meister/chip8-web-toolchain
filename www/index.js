import 'bootstrap'
import 'bootstrap/dist/css/bootstrap.min.css';
import css from "./index.css"
import { Chip8 } from "../pkg/c8_web_toolchain";
import { emulator_view } from './emulator_view.js';
import Split from 'split.js'
import ace from './ace-builds/src-noconflict/ace';
import "./ace-builds/src-noconflict/mode-c8i";
import "./ace-builds/webpack-resolver";

Split(['#split-0', '#split-1'], {
    sizes: [125, 125],
    direction: 'vertical',
    onDrag: onDragVertical,
})

Split(['#split-2', '#split-3'], {
    onDrag: onDragHorizontal,
})

Split(['#split-4', '#split-5'])

var rom_dropdown = document.getElementById("select-rom");
rom_dropdown.addEventListener("change", function(e) {
    view.load_rom_from_file(e.target.value);
});

var editor_split_parent = document.getElementById("split-3");
var editor_div = document.getElementById("editor");
editor_div.setAttribute("style", `width:${editor_split_parent.offsetWidth * 0.95}px`);
editor_div.setAttribute("style", `height:${editor_split_parent.offsetHeight * 0.5}px`);

var editor = ace.edit("editor");
editor.setValue('var numdrawcalls = 10; \nvar drawdelay = 50;\n\nfn drawrand(drawcount, delay) {\n\t I = 20;\n\t while(drawcount != 0) {\n\t\t drawcount = drawcount - 1;\n\t\t DT = delay;\n\t\t while (DT != 0) {}\n\t\t DRAW(RAND(255), RAND(255), 5);\n\t }\n }\n\n drawrand(numdrawcalls, drawdelay);\n while(1 == 1) {}', -1);
editor.session.setUseWrapMode(true);
editor.setTheme("ace/theme/chaos");
editor.session.setMode("ace/mode/c8i");
editor.resize();

onDragHorizontal();

let c8 = Chip8.new();
let view = new emulator_view(c8, 11, editor,
    document.getElementById("split-0"),
    document.getElementById("split-4"),
    document.getElementById("split-5"),
    document.getElementById("editor-container"),
);

editor.addEventListener("changeSelection", function() {
    view.draw_disasm();
});

function onDragVertical(sizes) {
    console.log(sizes[0]);
    view.set_scale(Math.ceil(view.scale * (sizes[0] / 100)));
    editor.resize();
}

function onDragHorizontal() {
    editor_div.setAttribute("style", `width:${editor_split_parent.offsetWidth * 0.95}px`);
    editor.resize();
}

(async () => {
    await view.load_rom_from_file('pong.rom');
    view.emulation_loop();
})();