// Two combinational modules for the `ev synth --design` path.
//
// `decode_demo` matches the file stem, so it is the resolved top module when
// `--top` is omitted, and `decode_demo_helper` exists so the `--top` override
// can be exercised against a module whose name differs from the file.
//
// The shapes are deliberately small: this fixture covers the design-only CLI
// path and its top-module selection, not any hardware contract. The hardware
// contracts live in the fixture specs under tests/fixtures/, and the Tagma
// decoder RTL itself lives in syntagma.

module decode_demo (
    input  wire [7:0] code,
    output wire       valid,
    output wire [3:0] group,
    output wire [3:0] index
);
    wire [7:0] shifted = code + 8'h01;

    assign valid = (code >= 8'h80);
    assign group = shifted[7:4];
    assign index = shifted[3:0] ^ {4{valid}};
endmodule

module decode_demo_helper (
    input  wire [7:0] data,
    output wire [7:0] rotated
);
    assign rotated = {data[6:0], data[7]};
endmodule
