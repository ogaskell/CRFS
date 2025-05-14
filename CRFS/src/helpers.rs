pub fn get_named_arg(args: &Vec<String>, short: Option<&str>, long: Option<&str>) -> Option<String> {
    if short == None && long == None {panic!("Must provide short or long argument name.")}

    let mut short_pre = String::from("-");
    let mut long_pre = String::from("--");

    let search_s = match short {
        Some(s) => {short_pre.push_str(&s); args.iter().position(|x| *x == short_pre)},
        None => None,
    };

    let search_l = match long {
        Some(l) => {long_pre.push_str(&l); args.iter().position(|x| *x == long_pre)},
        None => None,
    };

    let i = match (search_s, search_l) {
        (Some(i), _) => i,
        (None, Some(i)) => i,
        (None, None) => return None,
    };

    let s = args.get(i + 1)?;
    return Some(s.clone());
}

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
