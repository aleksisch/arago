use super::config::DIMENSION;
use ndarray::{ArrayView, ArrayView2, ArrayViewMut2, Ix1};
use std::cmp::max;
use std::iter::zip;
use std::ops::{Index, IndexMut};

type ChipT = i8;

pub struct Array1D {
    data: [ChipT; DIMENSION],
    sz: usize,
}

impl Index<usize> for Array1D
{
    type Output = ChipT;

    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.sz);
        self.data.index(index)
    }
}

pub struct Matrix {
    data: [ChipT; DIMENSION * DIMENSION],
    rows: usize,
    cols: usize,
}

impl Matrix {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Matrix {
            data: [0; DIMENSION * DIMENSION],
            rows,
            cols,
        }
    }

    fn at(&self, row: usize, col: usize) -> ChipT {
        assert!(row < self.rows);
        assert!(col < self.cols);
        self.data[row * DIMENSION + col]
    }

    pub fn convert(&self, res: &mut ArrayViewMut2<f32>) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                res[[row, col]] = scaled_to_f32(self.at(row, col) as i16);
            }
        }
    }

}

impl Index<(usize, usize)> for Matrix
{
    type Output = ChipT;

    #[inline(always)]
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.data[index.0 * DIMENSION + index.1]
    }
}


impl IndexMut<(usize, usize)> for Matrix
{
    #[inline(always)]
    fn index_mut(&mut self, index: (usize, usize)) -> &mut ChipT {
        assert!(index.0 < self.rows);
        assert!(index.1 < self.cols);
        &mut self.data[index.0 * DIMENSION + index.1]
    }
}


impl<'a> TryFrom<ArrayView<'a, f32, Ix1>> for Array1D {
    type Error = &'static str;

    fn try_from(value: ArrayView<'a, f32, Ix1>) -> Result<Self, Self::Error> {
        if value.shape().len() > DIMENSION {
            Err("Tried to create OPAC array from higher dimension")
        } else {
            let mut d: [i8; DIMENSION] = [0;DIMENSION];
            let values = value
                .map(|x| f32_to_chip(*x))
                .into_iter();
            for (dst, src) in zip(&mut d, values) {
                *dst = src;
            }
            Ok(Array1D {
                data: d,
                sz: DIMENSION,
            })
        }
    }
}

impl<'a> TryFrom<ArrayView2<'a, f32>> for Matrix {
    type Error = &'static str;

    fn try_from(value: ArrayView2<'a, f32>) -> Result<Self, Self::Error> {
        if value.shape().len() > DIMENSION {
            Err("Tried to create OPAC array from higher dimension")
        } else {
            let mut d = [0; DIMENSION * DIMENSION];
            for i in 0..value.shape()[0] {
                for j in 0..value.shape()[1] {
                    d[i * DIMENSION + j] = f32_to_chip(value[[i, j]]);
                }
            }
            Ok(Matrix {
                data: d,
                rows: value.shape()[0],
                cols: value.shape()[1],
            })
        }
    }
}


fn f32_to_chip(x: f32) -> ChipT {
    (x * 128.0).round() as i8
}

fn scaled_to_f32(x: i16) -> f32 {
    println!("Back {}", x);
    x as f32 / 128.0 / 128.0
}

pub fn opac(res: &mut Matrix, a: Array1D, b: Array1D) {
    for i in 0..res.rows {
        for j in 0..res.cols {
            println!("{} {} {}", i, j, a[i] * b[j]);
            res[(j, i)] += a[i] * b[j];
        }
    }
}

pub fn sca_mul(a: &Array1D, b: &Array1D) -> Array1D {
    let mut res = [0; DIMENSION];
    for (i, (a, b)) in zip(a.data, b.data).enumerate() {
        res[i] = a * b;
    }
    Array1D { data: res, sz: DIMENSION }
}

pub fn v_min(a: &Array1D, b: &Array1D) -> Array1D {

    let mut res = [0; DIMENSION];
    for (i, (a, b)) in zip(a.data, b.data).enumerate() {
        res[i] = max(a, b);
    }
    Array1D { data: res, sz: DIMENSION }
}

pub fn v_max(a: &Array1D, b: &Array1D) -> Array1D {
    let mut res = [0; DIMENSION];
    for (i, (a, b)) in zip(a.data, b.data).enumerate() {
        res[i] = max(a, b);
    }
    Array1D { data: res, sz: DIMENSION }
}
