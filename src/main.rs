fn main() {
    let a = "1234567";
    let mut iter = a.chars();
    for _ in 0..3 {
        iter.next();
    }
    let b: String = iter.collect();
    println!("{b}");
}
