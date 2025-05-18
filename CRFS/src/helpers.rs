pub fn lcs<T>(x: &[T], y: &[T]) -> Vec<T> where T: Eq + Clone {
    if x.len() == 0 || y.len() == 0 {return Vec::new();}
    if x[x.len() - 1] == y[y.len() - 1] {
        let mut result = Vec::from(lcs(&x[0..x.len() - 1], &y[0..y.len() - 1]));
        result.push(x[x.len() - 1].clone());
        return result;
    }

    let a = lcs(&x[0..x.len()], &y[0..y.len() - 1]);
    let b = lcs(&x[0..x.len() - 1], &y[0..y.len()]);

    if a.len() > b.len() {return a;} else {return b;}
}
