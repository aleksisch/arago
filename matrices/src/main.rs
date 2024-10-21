use ndarray::array;
use crate::intrinsics::wrappers::mat_mul;

mod intrinsics;

fn main() {
    let denom = 64.;
    let a = array![
            [1. / denom, 2. / denom],
            [3. / denom, 4. / denom]
        ];
    let b = array![
            [1. / denom, 2. / denom],
            [3. / denom, 4. / denom]
        ];

    let res = mat_mul(a.t().view(), b.view());
    println!("{:?}", res);
}
