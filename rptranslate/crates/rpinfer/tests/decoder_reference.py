"""Independent FP64 NumPy fixture, development only."""
import numpy as np

x = np.array([[1., 2., 3., 4.], [2., 0., 1., -1.], [3., -2., 1., 2.]])
memory = np.array([[1., 0., 2., 1.], [0., 1., 1., 0.], [9., 8., 7., 6.]])
wq = np.diag([1, 2, 1, .5])
wk = np.eye(4)
wv = np.diag([2, 1, .5, 1])
wo = np.array([[1, 0, 1, 0], [0, 1, 0, 1], [1, 0, 0, 0], [0, 0, 0, 1]])
w1 = np.array([[1, -1, 0, 0], [0, 1, 1, 0], [0, 0, -1, 1]])
w2 = np.array([[1, 0, .5], [0, -1, 0], [.5, 1, 0], [0, 0, 1]])
gain = np.array([1, 1.5, .5, 1])
bias = np.array([0, .1, 0, -.1])
b1 = np.array([.2, -.1, .3])
b2 = np.array([.1, 0, -.2, .1])


def norm(z):
    return (z-z.mean(-1, keepdims=True))/np.sqrt(
        z.var(-1, keepdims=True)+1e-5
    )*gain+bias


def attention(x, memory, mask):
    q, k, v = x @ wq.T, memory @ wk.T, memory @ wv.T
    parts = []
    for i in (0, 2):
        scores = q[:, i:i+2] @ k[:, i:i+2].T / np.sqrt(2)
        scores = np.where(mask, scores, -np.inf)
        p = np.exp(scores - scores.max(1, keepdims=True))
        p /= p.sum(1, keepdims=True)
        parts.append(p @ v[:, i:i+2])
    return np.concatenate(parts, axis=1) @ wo.T


for i in range(2):
    a = norm(x + attention(x, x, np.tril(np.ones((3, 3), bool))))
    c = norm(a + attention(a, memory, [[True, True, False]]))
    x = norm(c + np.maximum(c @ w1.T + b1, 0) @ w2.T + b2)
    for name, value in (("self", a), ("cross", c), ("output", x)):
        print(i, name, ", ".join(f"{v:.6f}" for v in value.ravel()))
