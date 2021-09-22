extern crate libc;
use std::ptr;
use std::fmt;
use crate::errors::FlotonErr;
use crate::traits::*;

pub fn unix_time() -> libc::time_t {
	unsafe { libc::time(ptr::null_mut()) }
}

// 08-19-2021 16:50:15

pub struct DateTime {
	data:libc::tm
}

impl NewType for DateTime {
	fn new() -> Self {
		DateTime{data:libc::tm{tm_sec:0, tm_min:0, tm_hour:0, 
		                   tm_mday:0, tm_mon:0, tm_year:0,
		                   tm_wday:0, tm_yday:0, tm_isdst:0,
		                   tm_gmtoff:0, tm_zone:ptr::null_mut()}}
	}
}

impl DateTime {
	pub fn set(&mut self, time_point:&libc::time_t) -> Result<(), FlotonErr> {
		if isnull!(unsafe { libc::gmtime_r(time_point, &mut self.data) } ) {
			Err(FlotonErr::DateTime)
		} else {
			self.data.tm_year += 1900; // tm_year is relative to 1900
			self.data.tm_mon += 1; // 0-11 mon
			Ok(())
		}
	}

	#[inline]
	pub fn set_to_now(&mut self) -> Result<(), FlotonErr> {
		self.set(&unix_time())
	}
}

impl fmt::Display for DateTime {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}-{}{}-{} {}{}:{}{}:{}{}", self.data.tm_mon / 10, self.data.tm_mon % 10,
        	                                       self.data.tm_mday / 10, self.data.tm_mday % 10,
        	                                       self.data.tm_year,
        	                                       self.data.tm_hour / 10, self.data.tm_hour % 10,
        	                                       self.data.tm_min / 10, self.data.tm_min % 10,
        	                                       self.data.tm_sec / 10, self.data.tm_sec % 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn dt_set_works() {
    	let test_time = 1621100000; // Saturday, May 15, 2021 5:33:20 PM
    	let mut dt = DateTime::new();
    	dt.set(&test_time).expect("Could not set to desired time");
    	assert_eq!(dt.data.tm_hour, 17);
    	assert_eq!(dt.data.tm_year, 2021);
    	assert_eq!(dt.data.tm_min, 33);
    	assert_eq!(dt.data.tm_sec, 20);
    	assert_eq!(dt.data.tm_mon, 5);
    	assert_eq!(dt.data.tm_mday, 15);
    }

    #[test]
    fn dt_display_works() {
    	let test_time = 1621100000;
    	let mut dt = DateTime::new();
    	dt.set(&test_time).expect("Could not set to desired time");
    	println!("Checking the datetime: {}", dt);
    	let fmtted = format!("{}", dt);
    	let fmt_bytes = fmtted.as_bytes();
    	assert_eq!(fmt_bytes[0], '0' as u8);
    	assert_eq!(fmt_bytes[1], '5' as u8);
    	assert_eq!(fmt_bytes[2], '-' as u8);
    	assert_eq!(fmt_bytes[3], '1' as u8);
    	assert_eq!(fmt_bytes[4], '5' as u8);
    	assert_eq!(fmt_bytes[5], '-' as u8);
    	assert_eq!(fmt_bytes[6], '2' as u8);
    	assert_eq!(fmt_bytes[7], '0' as u8);
    	assert_eq!(fmt_bytes[8], '2' as u8);
    	assert_eq!(fmt_bytes[9], '1' as u8);
    	assert_eq!(fmt_bytes[10], ' ' as u8);
    	assert_eq!(fmt_bytes[11], '1' as u8);
    	assert_eq!(fmt_bytes[12], '7' as u8);
    	assert_eq!(fmt_bytes[13], ':' as u8);
    	assert_eq!(fmt_bytes[14], '3' as u8);
    	assert_eq!(fmt_bytes[15], '3' as u8);
    	assert_eq!(fmt_bytes[16], ':' as u8);
    	assert_eq!(fmt_bytes[17], '2' as u8);
    	assert_eq!(fmt_bytes[18], '0' as u8);
    }
}
