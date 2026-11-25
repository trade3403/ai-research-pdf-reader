"""Independent NumPy FP64 reference for the synthetic joint model."""
import numpy as np

table = np.array([[0.,1.,2.,3.],[1.,-1.,.5,2.],[2.,1.,0.,-1.],
                  [-1.,2.,1.,0.],[.5,-.5,1.,-1.]])
q = np.diag([1.,2.,1.,.5])
k = np.eye(4)
v = np.diag([2.,1.,.5,1.])
o = np.array([[1,0,1,0],[0,1,0,1],[1,0,0,0],[0,0,0,1.]])
w1 = np.array([[1,-1,0,0],[0,1,1,0],[0,0,-1,1.]])
w2 = np.array([[1,0,.5],[0,-1,0],[.5,1,0],[0,0,1.]])
b1 = np.array([.2,-.1,.3])
b2 = np.array([.1,0,-.2,.1])
gain = np.array([1,1.5,.5,1])
shift = np.array([0,.1,0,-.1])
bias = np.array([.1,-.2,0,.3,-.1])


def norm(z):
    return (z-z.mean(-1,keepdims=True))/np.sqrt(
        z.var(-1,keepdims=True)+1e-5)*gain+shift


def emb(ids):
    a = np.arange(len(ids))[:,None]/(10000**(2*np.arange(2)/4))
    return table[ids]*2+np.concatenate([np.sin(a),np.cos(a)],1)


def att(x, m, mask):
    Q, K, V = x@q.T, m@k.T, m@v.T
    parts = []
    for j in (0,2):
        s = Q[:,j:j+2]@K[:,j:j+2].T/np.sqrt(2)
        s = np.where(mask,s,-np.inf)
        p = np.exp(s-s.max(1,keepdims=True))
        p /= p.sum(1,keepdims=True)
        parts.append(p@V[:,j:j+2])
    return np.concatenate(parts,1)@o.T


def ff(x):
    return norm(x+np.maximum(x@w1.T+b1,0)@w2.T+b2)


x = emb([1,2,4])
mem = ff(norm(x+att(x,x,[[True,True,False]])))
y = emb([4,3])
a = norm(y+att(y,y,np.tril(np.ones((2,2),bool))))
dec = ff(norm(a+att(a,mem,[[True,True,False]])))
logits = dec@table.T+bias
prob = np.exp(logits-logits.max(1,keepdims=True))
prob /= prob.sum(1,keepdims=True)
for name,z in [('memory',mem),('hidden',dec),('logits',logits),('prob',prob)]:
    print(name, ', '.join(f'{v:.6f}' for v in z.ravel()))
