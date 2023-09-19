pub fn calculate_hrv(rr_intervals: &Vec<u16>) -> (f32, f32) {
    // Calculate SDNN
    let mean_rr = rr_intervals.iter().map(|&x| x as f32).sum::<f32>() / rr_intervals.len() as f32;
    let sdnn = (rr_intervals.iter().map(|x| {
        let diff = *x as f32 - mean_rr;
        diff * diff
    }).sum::<f32>() / rr_intervals.len() as f32).sqrt();
    // Calculate RMSSD
    let mut diffs = Vec::new();
    for i in 0..rr_intervals.len() - 1 {
        let diff = rr_intervals[i + 1] as f32 - rr_intervals[i] as f32;
        diffs.push(diff * diff);
    }
    let rmssd = (diffs.iter().sum::<f32>() / diffs.len() as f32).sqrt();
    // return
    (sdnn, rmssd)
}
