use std::cmp::min;
use std::iter::zip;
use ndarray::{s, Array2, ArrayView2, ArrayViewMut2};
use crate::intrinsics::config::DIMENSION;
use super::intrinsics::{opac, Matrix};

/// Makes blocks of size no more than DIMENSION multiplication
fn block_mul<'a>(mut res: ArrayViewMut2<f32>, a: ArrayView2<'a, f32>, b: ArrayView2<'a, f32>) {
    assert!(a.shape()[1] <= DIMENSION);
    assert!(b.shape()[1] <= DIMENSION);
    assert_eq!(a.shape()[1], b.shape()[1]);
    assert_eq!(res.shape()[0], a.shape()[0]);
    assert_eq!(res.shape()[1], b.shape()[0]);

    let mut res_mat = Matrix::zeros(a.nrows(), b.nrows());
    for (r1, r2) in zip(a.columns(), b.columns()) {
        opac(&mut res_mat, r1.try_into().unwrap(), r2.try_into().unwrap());
    }
    res_mat.convert(&mut res);
}


/// Makes any shape matrix multiplication
pub fn mat_mul<'a>(a: ArrayView2<'a, f32>, b: ArrayView2<'a, f32>) -> Array2<f32> {
    assert_eq!(a.shape()[1], b.shape()[1]);
    let common_dim = a.shape()[1];
    let mut res = Array2::default([a.shape()[0], b.shape()[0]]);
    for i in (0..a.shape()[0]).step_by(DIMENSION) {
        for j in (0..b.shape()[0]).step_by(DIMENSION) {
            for k in (0..common_dim).step_by(DIMENSION) {
                let next_i = min(i + DIMENSION, a.shape()[0]);
                let next_j = min(j + DIMENSION, b.shape()[0]);
                let next_k = min(k + DIMENSION, a.shape()[1]);

                let a_index = s![i..next_i, k..next_k];
                let b_index = s![j..next_j, k..next_k];
                let res_index = s![i..next_i, j..next_j];

                let block_a = a.slice(a_index);
                let block_b = b.slice(b_index);



                let res_block = res.slice_mut(res_index);
                block_mul(res_block, block_a, block_b);
            }
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use ndarray::array;
    use super::*;

    #[test]
    fn simple_mat_mul() {
        let denom = 64.;
        let a = array![
            [1. / denom, 2. / denom],
            [3. / denom, 4. / denom]
        ];
        let b = array![
            [1. / denom, 2. / denom],
            [3. / denom, 4. / denom]
        ];
        let c = array![
            [7. / denom / denom, 10. / denom / denom],
            [15. / denom / denom, 22. / denom / denom]];
        let res = mat_mul(a.view().t(), b.view());
        let sum = (res - c).sum().abs();
        assert!(sum < f32::EPSILON);
    }
}
