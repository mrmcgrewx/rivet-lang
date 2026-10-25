use std::collections::HashMap;

pub fn solve(nums: &[i64], target: i64) -> (usize, usize) {
    let mut seen = HashMap::with_capacity(nums.len());

    for (index, &value) in nums.iter().enumerate() {
        let complement = target - value;
        if let Some(&previous_index) = seen.get(&complement) {
            return (previous_index, index);
        }
        seen.insert(value, index);
    }

    panic!("input must contain a pair summing to target");
}
