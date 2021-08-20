use std::ops::{Index, IndexMut};
use std::iter::IntoIterator;


pub struct Matrix<F> {
    data: Vec<F>,
    height: usize,
    width: usize,
}

pub struct MatrixRow<'a, F> {
    matrix: &'a Matrix<F>,
    i: usize,
}

pub struct MatrixCol<'a, F> {
    matrix: &'a Matrix<F>,
    j: usize,
}


impl<F> Matrix<F> {
    pub fn get_row(&self, i: usize) -> Option<MatrixRow<'_, F>> {
        if i < self.height {
            Some(MatrixRow {matrix: &self, i})
        } else {
            None
        }
    }

    pub fn get_col(&self, j: usize) -> Option<MatrixCol<'_, F>> {
        if j < self.width {
            Some(MatrixCol {matrix: &self, j})
        } else {
            None
        }
    }

    pub fn get<'a>(&'a self, (i, j): (usize, usize)) -> Option<&'a F> {
        if i < self.height && j < self.width {
            Some(&self.data[i * self.width + j])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, (i, j): (usize, usize)) -> Option<&mut F> {
        if i < self.height && j < self.width {
            Some(&mut self.data[i * self.width + j])
        } else {
            None
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.height, self.width)
    }

    pub fn iter_rows<'a>(&'a self) -> impl ExactSizeIterator<Item = MatrixRow<'a, F>> + 'a {
        (0..self.height).map(move |i| self.get_row(i).unwrap())
    }

    pub fn iter_cols<'a>(&'a self) -> impl ExactSizeIterator<Item = MatrixCol<'a, F>> + 'a {
        (0..self.width).map(move |i| self.get_col(i).unwrap())
    }
}

impl<F: Copy> Matrix<F> {
    pub fn to_nested_vec(&self) -> Vec<Vec<F>> {
        (0..self.height).map(|i| self.get_row(i).unwrap().to_vec()).collect()
    }
}

impl<F: Copy> From<&Vec<Vec<F>>> for Matrix<F> {
    fn from(v: &Vec<Vec<F>>) -> Matrix<F> {
        let height = v.len();
        let width = v.get(0).expect("unknown matrix width").len();
        if v.iter().any(|row| row.len() != width) {
            panic!("inconsistent row width");
        }

        let mut data: Vec<F> = Vec::with_capacity(height * width);
        v.iter().for_each(|row| data.extend(row));

        Self { data, height, width }
    }
}

impl<F> Index<(usize, usize)> for Matrix<F> {
    type Output = F;
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        self.get(index).expect("out of bounds")
    }
}

impl<F> IndexMut<(usize, usize)> for Matrix<F> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        self.get_mut(index).expect("out of bounds")
    }
}


impl<F> MatrixRow<'_, F> {
    pub fn get(&self, j: usize) -> Option<&F> {
        self.matrix.get((self.i, j))
    }
    pub fn size(&self) -> usize {
        self.matrix.width
    }
    pub fn iter<'a>(&'a self) -> impl ExactSizeIterator<Item = &F> + 'a {
        (0..self.matrix.width).map(move |j| &self[j])
    }
}

impl<F: Copy> MatrixRow<'_, F> {
    pub fn to_vec(&self) -> Vec<F> {
        (0..self.matrix.width).map(|j| self[j]).collect()
    }
}

impl<F> Index<usize> for MatrixRow<'_, F> {
    type Output = F;
    fn index(&self, j: usize) -> &Self::Output {
        self.get(j).expect("out of bounds")
    }
}


impl<F> MatrixCol<'_, F> {
    pub fn get(&self, i: usize) -> Option<&F> {
        self.matrix.get((i, self.j))
    }
    pub fn size(&self) -> usize {
        self.matrix.height
    }
    pub fn iter<'a>(&'a self) -> impl ExactSizeIterator<Item = &F> + 'a {
        (0..self.matrix.height).map(move |i| &self[i])
    }
}

impl<F: Copy> MatrixCol<'_, F> {
    pub fn to_vec(&self) -> Vec<F> {
        (0..self.matrix.height).map(|i| self[i]).collect()
    }
}

impl<F> Index<usize> for MatrixCol<'_, F> {
    type Output = F;
    fn index(&self, i: usize) -> &Self::Output {
        self.get(i).expect("out of bounds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_from() {
        let vec_matrix: Vec<Vec<i8>> = vec![(0..4).collect(), (4..8).collect(), (8..12).collect()];
        let matrix = Matrix::from(&vec_matrix);

        for i in 0..vec_matrix.len() {
            for j in 0..vec_matrix[0].len() {
                assert_eq!(matrix[(i, j)], vec_matrix[i][j]);
            }
        }
    }

    #[test]
    fn test_matrix_to() {
        let vec_matrix_goal: Vec<Vec<i8>> = vec![
            (0..3).collect(),
            (3..6).collect(),
            (6..9).collect(),
            (9..12).collect(),
            (12..15).collect(),
        ];

        let mut data: Vec<i8> = vec![];
        vec_matrix_goal.iter().for_each(|v| data.extend(v));

        let matrix = Matrix {data, height: vec_matrix_goal.len(), width: vec_matrix_goal[0].len()};
        let vec_matrix_res = matrix.to_nested_vec();

        assert_eq!(vec_matrix_res, vec_matrix_goal);
    }

    #[test]
    fn test_matrix_index() {
        let data: Vec<i8> = (0..12).collect();
        let height: usize = 3;
        let width: usize = 4;
        let matrix = Matrix {data, height, width};

        assert_eq!(matrix[(0, 0)], 0);
        assert_eq!(matrix[(0, 3)], 3);
        assert_eq!(matrix[(2, 0)], 8);
        assert_eq!(matrix[(2, 3)], 11);
    }

    #[test]
    fn test_matrix_index_mut() {
        let data: Vec<i8> = (0..12).collect();
        let height: usize = 3;
        let width: usize = 4;
        let mut matrix = Matrix {data, height, width};

        assert_eq!(matrix[(0, 0)], 0);
        assert_eq!(matrix[(0, 3)], 3);
        assert_eq!(matrix[(2, 0)], 8);
        assert_eq!(matrix[(2, 3)], 11);

        matrix[(0, 0)] = 12;
        matrix[(0, 3)] = 13;
        matrix[(2, 0)] = 14;
        matrix[(2, 3)] = 15;

        assert_eq!(matrix[(0, 0)], 12);
        assert_eq!(matrix[(0, 3)], 13);
        assert_eq!(matrix[(2, 0)], 14);
        assert_eq!(matrix[(2, 3)], 15);
    }

}
