

/**
 * Runs some code and takes the average time
 */
macro_rules! average_s {
    ($name:ident, $times:expr, $code:block) => {
        {
        	log_always!($name, "Running benchmark {:?} times", $times);
        	let mut total = 0.0;
        	for i in 0..$times {
        		let start = Instant::now();
        		$code
        		let lap = start.elapsed().as_secs_f64();
        		log_always!($name, "Trial = {} time in seconds = {}", i, lap);
        		total += lap;
        	}
        	let avg = total / (($times) as f64);
        	log_always!($name, "Completed, average in seconds = {}", avg);
        }
    };

    ($times:expr, $code:block) => {
        {
        	let mut total = 0.0;
        	for _ in 0..$times {
        		let start = Instant::now();
        		$code
        		total += start.elapsed().as_secs_f64();
        	}
        	total / (($times) as f64)
        }
    };
}
