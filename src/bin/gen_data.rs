fn main() {
    let words = ["foo", "bar", "baz", "qux", "hello", "world", "rust", "awk"];
    for i in 1..=1_000_000 {
        let w1 = words[i % words.len()];
        let w2 = words[(i * 3) % words.len()];
        let w3 = words[(i * 7) % words.len()];
        println!("{} {} {} {}", i, w1, w2, w3);
    }
}