"""Leaf-evaluator training pipeline (Post-P3).

Dev-side package: trains the residual MLP that prices resolver leaf nodes,
exports it to ONNX, and verifies runtime parity. Only :mod:`model` imports
torch at module load time; :mod:`dataset`, :mod:`train`, :mod:`export`, and
:mod:`calibrate` defer heavyweight imports to call time so the shipped wheel
keeps this package's modules out of the default import path.
"""
