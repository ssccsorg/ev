// Synthesis test fixture: ck_even
// parity check -- 1 if coord is even, 0 if odd
//
// Intended to exercise the Yosys synthesis channel with a minimal
// combinational module. Equivalent gate-level result: 1 LUT, 0 registers.

module ck_even (
    input  logic [63:0] coord,
    output logic        result
);

    assign result = ~coord[0];

endmodule
