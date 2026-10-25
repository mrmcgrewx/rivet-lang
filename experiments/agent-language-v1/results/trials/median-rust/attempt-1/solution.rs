pub fn solve(first: &[i64], second: &[i64]) -> f64 {
    let (short, long) = if first.len() <= second.len() {
        (first, second)
    } else {
        (second, first)
    };
    let m = short.len();
    let n = long.len();
    let total = m + n;
    let left_len = (total + 1) / 2;
    let mut low = 0;
    let mut high = m;

    // Searching the shorter array keeps the other partition within 0..=n.
    while low <= high {
        let i = low + (high - low) / 2;
        let j = left_len - i;

        if i > 0 && j < n && short[i - 1] > long[j] {
            high = i - 1;
        } else if j > 0 && i < m && long[j - 1] > short[i] {
            low = i + 1;
        } else {
            let left_max = if i == 0 {
                long[j - 1]
            } else if j == 0 {
                short[i - 1]
            } else {
                short[i - 1].max(long[j - 1])
            };

            if total % 2 == 1 {
                return left_max as f64;
            }

            let right_min = if i == m {
                long[j]
            } else if j == n {
                short[i]
            } else {
                short[i].min(long[j])
            };
            return (left_max as f64 + right_min as f64) / 2.0;
        }
    }

    unreachable!("valid sorted inputs always admit a median partition")
}
