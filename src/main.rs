use floton::tlocal;
use floton::datetime;

fn main() {
    println!("---- Floton DB ----");
    println!("TID: {}", tlocal::tid());
    println!("Unix Time: {}", datetime::unix_time());
}
