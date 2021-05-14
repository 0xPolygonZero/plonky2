#[macro_export]
macro_rules! timed {
    ($a:expr, $msg:expr) => {{
        use std::time::Instant;

        use log::info;

        let timer = Instant::now();
        let res = $a;
        info!("{:.3}s {}", timer.elapsed().as_secs_f32(), $msg);
        res
    }};
}
