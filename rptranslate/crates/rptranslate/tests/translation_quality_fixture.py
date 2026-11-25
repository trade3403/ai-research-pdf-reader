"""Shared original short-sentence quality fixture and observational metrics."""
import collections
import statistics
import unicodedata

# Original authored fixture, not a held-out corpus or human evaluation.
CASES = [
    ("The result is stable.", "结果稳定。", [], [("结果",), ("稳定",)]),
    ("The temperature is 25 degrees Celsius.", "温度为25摄氏度。", ["25"], [("温度",), ("摄氏度", "℃", "°C")]),
    ("The sample contained 10 mg of sodium chloride.", "样品含有10毫克氯化钠。", ["10"], [("毫克", "mg"), ("氯化钠",)]),
    ("The experiment was repeated 3 times.", "实验重复了3次。", ["3"], [("实验",), ("重复",)]),
    ("The model uses a neural network for classification.", "该模型使用神经网络进行分类。", [], [("神经网络",), ("分类",)]),
    ("The accuracy increased from 80% to 90%.", "准确率从80%提高到90%。", ["80", "90"], [("准确率", "准确度", "精确度"), ("%", "百分之")]),
]

def norm(text):
    return unicodedata.normalize("NFKC", text).casefold()

def char_f(text, ref):
    """Character n-gram F2, effective orders 1..6, no whitespace; not SacreBLEU."""
    a, b = ("".join(norm(s).split()) for s in (text, ref))
    ps, rs = [], []
    for n in range(1, min(6, len(a), len(b)) + 1):
        ca = collections.Counter(a[i:i+n] for i in range(len(a)-n+1))
        cb = collections.Counter(b[i:i+n] for i in range(len(b)-n+1))
        hit = sum((ca & cb).values())
        ps.append(hit / sum(ca.values()))
        rs.append(hit / sum(cb.values()))
    if not ps:
        return 0.
    p, r = statistics.mean(ps), statistics.mean(rs)
    return 100 * 5 * p * r / (4 * p + r) if p + r else 0.

