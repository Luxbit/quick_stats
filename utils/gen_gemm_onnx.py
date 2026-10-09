#!/usr/bin/env python3
"""Generate the weightless 1024x1024 GEMM model used by the Windows/Linux GPU
benchmark.

Both operand matrices are graph inputs (no weights baked in), so the .onnx is
only a few hundred bytes and is committed to the repo. Regenerate with:

    pip install onnx
    python3 utils/gen_gemm_onnx.py models/gemm_1024.onnx

The TFLOPS math in src/benchmark/gpu.rs assumes this exact geometry
(1024x1024 float32 MatMul, flops = 2 * size^3 per invocation).
"""
import sys

import onnx
from onnx import TensorProto, helper

SIZE = 1024


def main(out_path: str) -> None:
    a = helper.make_tensor_value_info("a", TensorProto.FLOAT, [SIZE, SIZE])
    b = helper.make_tensor_value_info("b", TensorProto.FLOAT, [SIZE, SIZE])
    c = helper.make_tensor_value_info("c", TensorProto.FLOAT, [SIZE, SIZE])
    node = helper.make_node("MatMul", ["a", "b"], ["c"], name="gemm_1024")
    graph = helper.make_graph(
        [node], "quick_stats_gemm_1024", [a, b], [c]
    )
    model = helper.make_model(
        graph,
        producer_name="quick_stats-gen_gemm_onnx",
        opset_imports=[helper.make_opsetid("", 13)],
    )
    # IRv8 graphs load in every ORT we care about (1.17+ per the forks' README)
    model.ir_version = 8
    onnx.checker.check_model(model)
    onnx.save(model, out_path)
    print(f"wrote {out_path}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "models/gemm_1024.onnx")
