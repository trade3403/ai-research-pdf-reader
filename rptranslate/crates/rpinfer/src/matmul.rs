//! D1.4: portable and AVX2-dispatched row-major FP32 matrix multiplication.
use crate::tensor::{buffer, layout, Error, Tensor};

const TILE: usize = 32;
const MAX_MACS: usize = 1 << 30;

#[derive(Clone, Copy, Debug)]
pub enum Kernel {
    Scalar,
    Blocked,
    Auto,
    Parallel(usize),
}

/// Multiply two rank-2 tensors. All kernels accumulate each cell in
/// increasing K order without explicit fused multiply-add.
pub fn matmul(a: &Tensor, b: &Tensor, kernel: Kernel) -> Result<Tensor, Error> {
    let &[m, k] = a.shape() else {
        return Err(Error::Shape);
    };
    let &[bk, n] = b.shape() else {
        return Err(Error::Shape);
    };
    if k != bk {
        return Err(Error::Shape);
    }
    if let Kernel::Parallel(threads) = kernel {
        if !(1..=16).contains(&threads) {
            return Err(Error::Limit);
        }
    }
    let (_, size) = layout(&[m, n])?;
    let macs = size.checked_mul(k).ok_or(Error::Limit)?;
    if macs > MAX_MACS {
        return Err(Error::Limit);
    }
    let mut out = buffer(size)?;
    out.resize(size, 0.0);
    if size != 0 && k != 0 {
        match kernel {
            Kernel::Scalar => scalar(a.data(), b.data(), &mut out, k, n),
            Kernel::Blocked => blocked(a.data(), b.data(), &mut out, m, k, n),
            Kernel::Auto => auto(a.data(), b.data(), &mut out, m, k, n),
            Kernel::Parallel(threads) => {
                parallel(a.data(), b.data(), &mut out, m, k, n, threads)?;
            }
        }
    }
    Tensor::new(&[m, n], out)
}

fn scalar(a: &[f32], b: &[f32], out: &mut [f32], k: usize, n: usize) {
    for (ar, cr) in a.chunks_exact(k).zip(out.chunks_exact_mut(n)) {
        for (j, cell) in cr.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (&x, br) in ar.iter().zip(b.chunks_exact(n)) {
                sum += x * br[j];
            }
            *cell = sum;
        }
    }
}

fn blocked(a: &[f32], b: &[f32], out: &mut [f32], m: usize, k: usize, n: usize) {
    blocked_with(a, b, out, m, k, n, |cr, br, x| {
        for (c, &y) in cr.iter_mut().zip(br) {
            *c += x * y;
        }
    });
}

fn blocked_with(
    a: &[f32],
    b: &[f32],
    out: &mut [f32],
    m: usize,
    k: usize,
    n: usize,
    update: impl Fn(&mut [f32], &[f32], f32),
) {
    for ii in (0..m).step_by(TILE) {
        for kk in (0..k).step_by(TILE) {
            for jj in (0..n).step_by(TILE) {
                let end_j = (jj + TILE).min(n);
                for i in ii..(ii + TILE).min(m) {
                    let cr = &mut out[i * n + jj..i * n + end_j];
                    for p in kk..(kk + TILE).min(k) {
                        let x = a[i * k + p];
                        let br = &b[p * n + jj..p * n + end_j];
                        update(cr, br, x);
                    }
                }
            }
        }
    }
}

/// True only when the compiled implementation and runtime CPU/OS support AVX2.
pub fn avx2_available() -> bool {
    #[cfg(all(target_arch = "x86_64", not(feature = "portable-only")))]
    {
        std::is_x86_feature_detected!("avx2")
    }
    #[cfg(not(all(target_arch = "x86_64", not(feature = "portable-only"))))]
    {
        false
    }
}

fn auto(a: &[f32], b: &[f32], out: &mut [f32], m: usize, k: usize, n: usize) {
    #[cfg(all(target_arch = "x86_64", not(feature = "portable-only")))]
    if avx2_available() {
        // SAFETY: runtime feature detection above includes OS vector-state support.
        unsafe { avx2_blocked(a, b, out, m, k, n) };
        return;
    }
    blocked(a, b, out, m, k, n);
}

#[cfg(all(target_arch = "x86_64", not(feature = "portable-only")))]
#[target_feature(enable = "avx2")]
unsafe fn avx2_blocked(a: &[f32], b: &[f32], out: &mut [f32], m: usize, k: usize, n: usize) {
    blocked_with(a, b, out, m, k, n, |cr, br, x| {
        // SAFETY: this closure runs only inside the AVX2-enabled function.
        unsafe { avx2_update(cr, br, x) };
    });
}

#[cfg(all(target_arch = "x86_64", not(feature = "portable-only")))]
#[target_feature(enable = "avx2")]
unsafe fn avx2_update(cr: &mut [f32], br: &[f32], x: f32) {
    use std::arch::x86_64::*;
    debug_assert_eq!(cr.len(), br.len());
    let (cv, ct) = cr.as_chunks_mut::<8>();
    let (bv, bt) = br.as_chunks::<8>();
    let x8 = _mm256_set1_ps(x);
    for (c, b) in cv.iter_mut().zip(bv) {
        // SAFETY: each chunk holds eight initialized f32 values; unaligned
        // accesses are supported, and mutable output does not alias input.
        unsafe {
            let y = _mm256_loadu_ps(b.as_ptr());
            let old = _mm256_loadu_ps(c.as_ptr());
            _mm256_storeu_ps(c.as_mut_ptr(), _mm256_add_ps(old, _mm256_mul_ps(x8, y)));
        }
    }
    for (c, &y) in ct.iter_mut().zip(bt) {
        *c += x * y;
    }
}

fn parallel(
    a: &[f32],
    b: &[f32],
    out: &mut [f32],
    m: usize,
    k: usize,
    n: usize,
    threads: usize,
) -> Result<(), Error> {
    let threads = threads.min(m);
    if threads <= 1 || m * n * k < 32768 {
        auto(a, b, out, m, k, n);
        return Ok(());
    }
    let rows = m.div_ceil(threads);
    std::thread::scope(|scope| {
        let mut jobs = Vec::new();
        for (ar, cr) in a.chunks(rows * k).zip(out.chunks_mut(rows * n)) {
            let count = cr.len() / n;
            let handle = std::thread::Builder::new()
                .spawn_scoped(scope, move || auto(ar, b, cr, count, k, n))
                .map_err(|_| Error::Worker)?;
            jobs.push(handle);
        }
        for handle in jobs {
            handle.join().map_err(|_| Error::Worker)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_reference() {
        let a = Tensor::new(&[2, 3], vec![1., 2., 3., 4., 5., 6.]).unwrap();
        let b = Tensor::new(&[3, 2], vec![7., 8., 9., 10., 11., 12.]).unwrap();
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            let c = matmul(&a, &b, kernel).unwrap();
            assert_eq!(c.shape(), &[2, 2]);
            assert_eq!(c.data(), &[58., 64., 139., 154.]);
        }
    }

    #[test]
    fn rectangular_tile_tails_and_f64_reference() {
        for (m, k, n) in [(1, 7, 3), (31, 33, 35), (33, 65, 17), (65, 32, 63)] {
            let a = Tensor::new(
                &[m, k],
                (0..m * k).map(|i| (i % 19) as f32 / 7.0 - 1.0).collect(),
            )
            .unwrap();
            let b = Tensor::new(
                &[k, n],
                (0..k * n).map(|i| (i % 23) as f32 / 11.0 - 1.0).collect(),
            )
            .unwrap();
            let scalar = matmul(&a, &b, Kernel::Scalar).unwrap();
            let blocked = matmul(&a, &b, Kernel::Blocked).unwrap();
            assert_eq!(scalar, blocked);
            assert_eq!(scalar, matmul(&a, &b, Kernel::Auto).unwrap());
            assert_eq!(scalar, matmul(&a, &b, Kernel::Parallel(3)).unwrap());
            for i in 0..m {
                for j in 0..n {
                    let reference: f64 = (0..k)
                        .map(|p| f64::from(a.data()[i * k + p]) * f64::from(b.data()[p * n + j]))
                        .sum();
                    let error = (f64::from(blocked.data()[i * n + j]) - reference).abs();
                    assert!(error <= 1e-5 + 1e-5 * reference.abs(), "error={error}");
                }
            }
        }
    }

    #[test]
    fn empty_and_invalid_shapes() {
        let a = Tensor::zeros(&[3, 0]).unwrap();
        let b = Tensor::zeros(&[0, 4]).unwrap();
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            assert_eq!(matmul(&a, &b, kernel).unwrap().data(), &[0.; 12]);
            let empty = Tensor::zeros(&[0, 3]).unwrap();
            let rhs = Tensor::zeros(&[3, 4]).unwrap();
            assert_eq!(matmul(&empty, &rhs, kernel).unwrap().shape(), &[0, 4]);
            let invalid = Tensor::zeros(&[4]).unwrap();
            assert_eq!(matmul(&a, &invalid, kernel), Err(Error::Shape));
            assert_eq!(matmul(&a, &rhs, kernel), Err(Error::Shape));
        }
    }

    #[test]
    fn parallel_dispatch_and_thread_limits() {
        let a = Tensor::new(&[73, 35], vec![0.125; 73 * 35]).unwrap();
        let b = Tensor::new(&[35, 41], vec![0.25; 35 * 41]).unwrap();
        let reference = matmul(&a, &b, Kernel::Scalar).unwrap();
        for threads in [1, 2, 3, 4, 16] {
            assert_eq!(
                reference,
                matmul(&a, &b, Kernel::Parallel(threads)).unwrap()
            );
        }
        assert_eq!(matmul(&a, &b, Kernel::Parallel(0)), Err(Error::Limit));
        assert_eq!(matmul(&a, &b, Kernel::Parallel(17)), Err(Error::Limit));
        #[cfg(feature = "portable-only")]
        assert!(!avx2_available());
    }

    #[test]
    fn resource_limit_and_overflow() {
        let a = Tensor::zeros(&[4096, 0]).unwrap();
        let b = Tensor::zeros(&[0, 4097]).unwrap();
        assert_eq!(matmul(&a, &b, Kernel::Blocked), Err(Error::Limit));
        let a = Tensor::new(&[1, 1], vec![f32::MAX]).unwrap();
        let b = Tensor::new(&[1, 1], vec![2.]).unwrap();
        for kernel in [
            Kernel::Scalar,
            Kernel::Blocked,
            Kernel::Auto,
            Kernel::Parallel(4),
        ] {
            assert_eq!(matmul(&a, &b, kernel), Err(Error::NonFinite));
        }
    }
}
