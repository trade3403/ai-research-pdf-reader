//! Dense, owned, row-major FP32 tensor with checked scalar operations.
//! No unsafe code, external runtime, or implicit view/aliasing.
const MAX_RANK: usize = 16;
const MAX_ELEMENTS: usize = 16 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Shape,
    Limit,
    Index,
    Axis,
    NonFinite,
    DivisionByZero,
    Allocation,
    Worker,
}

#[derive(Debug, PartialEq)]
pub struct Tensor {
    shape: Vec<usize>,
    strides: Vec<usize>,
    data: Vec<f32>,
}

pub(crate) fn layout(shape: &[usize]) -> Result<(Vec<usize>, usize), Error> {
    if shape.len() > MAX_RANK || shape.iter().any(|&d| d > MAX_ELEMENTS) {
        return Err(Error::Limit);
    }
    let mut strides = vec![0; shape.len()];
    let mut size = 1_usize;
    for (axis, &dim) in shape.iter().enumerate().rev() {
        strides[axis] = size;
        size = size.checked_mul(dim).ok_or(Error::Limit)?;
        if size > MAX_ELEMENTS {
            return Err(Error::Limit);
        }
    }
    Ok((strides, size))
}

pub(crate) fn buffer(size: usize) -> Result<Vec<f32>, Error> {
    let mut data = Vec::new();
    data.try_reserve_exact(size)
        .map_err(|_| Error::Allocation)?;
    Ok(data)
}

#[derive(Clone, Copy)]
pub enum Binary {
    Add,
    Sub,
    Mul,
    Div,
}

impl Tensor {
    pub fn new(shape: &[usize], data: Vec<f32>) -> Result<Self, Error> {
        let (strides, size) = layout(shape)?;
        if data.len() != size {
            return Err(Error::Shape);
        }
        if data.iter().any(|x| !x.is_finite()) {
            return Err(Error::NonFinite);
        }
        Ok(Self {
            shape: shape.to_vec(),
            strides,
            data,
        })
    }

    pub fn zeros(shape: &[usize]) -> Result<Self, Error> {
        let (_, size) = layout(shape)?;
        let mut data = buffer(size)?;
        data.resize(size, 0.0);
        Self::new(shape, data)
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    pub fn strides(&self) -> &[usize] {
        &self.strides
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }

    pub fn get(&self, indices: &[usize]) -> Result<f32, Error> {
        if indices.len() != self.shape.len() {
            return Err(Error::Index);
        }
        let mut offset = 0;
        for ((&index, &dim), &stride) in indices.iter().zip(&self.shape).zip(&self.strides) {
            if index >= dim {
                return Err(Error::Index);
            }
            offset += index * stride;
        }
        self.data.get(offset).copied().ok_or(Error::Index)
    }

    /// Changes only metadata, reuses the owned allocation.
    pub fn reshape(&mut self, shape: &[usize]) -> Result<(), Error> {
        let (strides, size) = layout(shape)?;
        if size != self.data.len() {
            return Err(Error::Shape);
        }
        self.shape = shape.to_vec();
        self.strides = strides;
        Ok(())
    }

    /// Materializes a contiguous result; axes must be a full permutation.
    pub fn permute(&self, axes: &[usize]) -> Result<Self, Error> {
        if axes.len() != self.shape.len() {
            return Err(Error::Axis);
        }
        let mut seen = [false; MAX_RANK];
        for &axis in axes {
            if axis >= axes.len() || seen[axis] {
                return Err(Error::Axis);
            }
            seen[axis] = true;
        }
        let shape: Vec<_> = axes.iter().map(|&axis| self.shape[axis]).collect();
        let (strides, size) = layout(&shape)?;
        let mut data = buffer(size)?;
        for flat in 0..size {
            let offset = axes
                .iter()
                .enumerate()
                .map(|(i, &axis)| flat / strides[i] % shape[i] * self.strides[axis])
                .sum::<usize>();
            data.push(self.data[offset]);
        }
        Self::new(&shape, data)
    }

    /// NumPy-style trailing-axis broadcasting, including scalar and empty axes.
    pub fn binary(&self, other: &Self, op: Binary) -> Result<Self, Error> {
        let rank = self.shape.len().max(other.shape.len());
        let mut shape = vec![1; rank];
        let mut left = vec![0; rank];
        let mut right = vec![0; rank];
        for axis in 0..rank {
            let (a, sa) = aligned(self, rank, axis);
            let (b, sb) = aligned(other, rank, axis);
            shape[axis] = if a == b || b == 1 {
                a
            } else if a == 1 {
                b
            } else {
                return Err(Error::Shape);
            };
            left[axis] = if a == 1 { 0 } else { sa };
            right[axis] = if b == 1 { 0 } else { sb };
        }
        let (strides, size) = layout(&shape)?;
        let mut data = buffer(size)?;
        for flat in 0..size {
            let mut a = 0;
            let mut b = 0;
            for axis in 0..rank {
                let coordinate = flat / strides[axis] % shape[axis];
                a += coordinate * left[axis];
                b += coordinate * right[axis];
            }
            let x = self.data[a];
            let y = other.data[b];
            let value = match op {
                Binary::Add => x + y,
                Binary::Sub => x - y,
                Binary::Mul => x * y,
                Binary::Div if y == 0.0 => return Err(Error::DivisionByZero),
                Binary::Div => x / y,
            };
            if !value.is_finite() {
                return Err(Error::NonFinite);
            }
            data.push(value);
        }
        Self::new(&shape, data)
    }
}

fn aligned(t: &Tensor, rank: usize, axis: usize) -> (usize, usize) {
    let leading = rank - t.shape.len();
    if axis < leading {
        (1, 0)
    } else {
        (t.shape[axis - leading], t.strides[axis - leading])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_stride_index_and_reshape() {
        let mut t = Tensor::new(&[2, 3], vec![1., 2., 3., 4., 5., 6.]).unwrap();
        assert_eq!(t.strides(), &[3, 1]);
        assert_eq!(t.get(&[1, 2]), Ok(6.));
        assert_eq!(t.get(&[0, 3]), Err(Error::Index));
        assert_eq!(t.get(&[0]), Err(Error::Index));
        let ptr = t.data().as_ptr();
        t.reshape(&[3, 2]).unwrap();
        assert_eq!(t.strides(), &[2, 1]);
        assert_eq!(ptr, t.data().as_ptr());
        assert_eq!(t.reshape(&[7]), Err(Error::Shape));
        assert_eq!(t.shape(), &[3, 2]);
    }

    #[test]
    fn numpy_reference_permutation() {
        let t = Tensor::new(&[2, 3, 4], (0..24).map(|x| x as f32).collect()).unwrap();
        let out = t.permute(&[2, 0, 1]).unwrap();
        assert_eq!(out.shape(), &[4, 2, 3]);
        assert_eq!(out.strides(), &[6, 3, 1]);
        assert_eq!(
            out.data(),
            &[
                0., 4., 8., 12., 16., 20., 1., 5., 9., 13., 17., 21., 2., 6., 10., 14., 18., 22.,
                3., 7., 11., 15., 19., 23.,
            ]
        );
        assert_eq!(t.permute(&[0, 0, 2]), Err(Error::Axis));
        assert_eq!(t.permute(&[0, 1, 3]), Err(Error::Axis));
        assert_eq!(t.permute(&[0, 1]), Err(Error::Axis));
    }

    #[test]
    fn numpy_reference_broadcast() {
        let a = Tensor::new(&[2, 1, 3], vec![1., 2., 3., 4., 5., 6.]).unwrap();
        let b = Tensor::new(&[1, 4, 1], vec![10., 20., 30., 40.]).unwrap();
        let sum = a.binary(&b, Binary::Add).unwrap();
        assert_eq!(sum.shape(), &[2, 4, 3]);
        assert_eq!(
            sum.data(),
            &[
                11., 12., 13., 21., 22., 23., 31., 32., 33., 41., 42., 43., 14., 15., 16., 24.,
                25., 26., 34., 35., 36., 44., 45., 46.,
            ]
        );
        let product = a.binary(&b, Binary::Mul).unwrap();
        assert_eq!(
            product.data(),
            &[
                10., 20., 30., 20., 40., 60., 30., 60., 90., 40., 80., 120., 40., 50., 60., 80.,
                100., 120., 120., 150., 180., 160., 200., 240.,
            ]
        );
    }

    #[test]
    fn scalar_empty_and_broadcast_errors() {
        let a = Tensor::new(&[2], vec![2., 4.]).unwrap();
        let s = Tensor::new(&[], vec![2.]).unwrap();
        assert_eq!(s.get(&[]), Ok(2.));
        assert_eq!(a.binary(&s, Binary::Div).unwrap().data(), &[1., 2.]);
        assert_eq!(s.binary(&a, Binary::Sub).unwrap().data(), &[0., -2.]);
        let empty = Tensor::zeros(&[0, 2]).unwrap();
        assert_eq!(empty.binary(&s, Binary::Add).unwrap().shape(), &[0, 2]);
        assert_eq!(empty.permute(&[1, 0]).unwrap().shape(), &[2, 0]);
        assert_eq!(empty.get(&[0, 0]), Err(Error::Index));
        let b = Tensor::zeros(&[3]).unwrap();
        assert_eq!(a.binary(&b, Binary::Add), Err(Error::Shape));
        let zero = Tensor::zeros(&[]).unwrap();
        assert_eq!(a.binary(&zero, Binary::Div), Err(Error::DivisionByZero));
    }

    #[test]
    fn rejects_overflow_and_nonfinite() {
        assert_eq!(Tensor::zeros(&[usize::MAX, 2]), Err(Error::Limit));
        assert_eq!(Tensor::zeros(&[MAX_ELEMENTS, 2]), Err(Error::Limit));
        assert_eq!(Tensor::zeros(&[1; MAX_RANK + 1]), Err(Error::Limit));
        assert_eq!(Tensor::new(&[2], vec![1.]), Err(Error::Shape));
        assert_eq!(Tensor::new(&[1], vec![f32::NAN]), Err(Error::NonFinite));
        let a = Tensor::new(&[], vec![f32::MAX]).unwrap();
        assert_eq!(a.binary(&a, Binary::Mul), Err(Error::NonFinite));
    }
}
