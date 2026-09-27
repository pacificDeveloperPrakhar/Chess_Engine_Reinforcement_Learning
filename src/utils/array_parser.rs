pub fn get_the_u64<T: PartialEq>(arr: &[T], null_value: T) -> u64 {
    assert!(arr.len() >= 64, "get_the_u64 requires at least 64 elements");
    let mut res: u64 = 0;
    for i in 0..8 {
        for j in 0..8 {
            if arr[i * 8 + j] != null_value {
                res |= 1u64 << (i * 8 + j);
            }
        }
    }
    res
}