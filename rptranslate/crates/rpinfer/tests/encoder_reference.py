"""Reproduce the independent FP64 encoder reference fixture (development only)."""
import numpy as np

x = np.array([[1, 2, 3, 4], [2, 0, 1, -1], [9, 8, 7, 6]], dtype=np.float64)
wq = np.diag([1, 2, 1, .5])
wk = np.eye(4)
wv = np.diag([2, 1, .5, 1])
wo = np.array([[1, 0, 1, 0], [0, 1, 0, 1], [1, 0, 0, 0], [0, 0, 0, 1]])
w1 = np.array([[1, -1, 0, 0], [0, 1, 1, 0], [0, 0, -1, 1]])
w2 = np.array([[1, 0, .5], [0, -1, 0], [.5, 1, 0], [0, 0, 1]])
b1 = np.array([.2, -.1, .3])
b2 = np.array([.1, 0, -.2, .1])
gain = np.array([1, 1.5, .5, 1])
bias = np.array([0, .1, 0, -.1])


def norm(y):
    return (y - y.mean(-1, keepdims=True)) / np.sqrt(
        y.var(-1, keepdims=True) + 1e-5
    ) * gain + bias


def layer(x):
    q, k, v = x @ wq.T, x @ wk.T, x @ wv.T
    parts = []
    for i in (0, 2):
        scores = q[:, i:i+2] @ k[:2, i:i+2].T / np.sqrt(2)
        p = np.exp(scores - scores.max(1, keepdims=True))
        p /= p.sum(1, keepdims=True)
        parts.append(p @ v[:2, i:i+2])
    a = norm(x + np.concatenate(parts, axis=1) @ wo.T)
    y = norm(a + np.maximum(a @ w1.T + b1, 0) @ w2.T + b2)
    return a, y


for i in range(2):
    a, x = layer(x)
    print("attention", a.astype(np.float32).ravel().tolist())
    print("output", x.astype(np.float32).ravel().tolist())
