use std::collections::HashMap;
use std::cmp::min;
use std::cmp::max;

use crate::apf::trace::*;
use crate::apf::histogram::Histogram;

/*
	Liveness Counter
	At each alloc or free operation, call alloc() and free() methods accordingly 
	Update timestep with inc_timer()
*/
pub struct LivenessCounter {
	n: usize, 	// Timer
	m: usize,	// Number of objects
	alloc_sum: Histogram,	// Sum of allocation times before time
	alloc_counts: Histogram,	// Number of allocations before time
	free_sum: Histogram,	// Sum of free times before time
	free_counts: Histogram,		// Number of frees before time
}

impl LivenessCounter {
	pub fn new() -> LivenessCounter {
		LivenessCounter {
			n: 1,		// Start at 1 or 0?
			m: 0,
			alloc_sum: Histogram::new(),		// Need to add anything at start?
			alloc_counts: Histogram::new(),
			free_sum: Histogram::new(),
			free_counts: Histogram::new()
		}
	}

	// Call whenever memory is allocated
	pub fn alloc(&mut self) {
		self.alloc_sum.add(self.n, self.n);
		self.alloc_counts.increment(self.n);
		self.m += 1;
	}

	// Call whenever memory is freed
	pub fn free(&mut self) {
		self.free_sum.add(self.n, self.n);
		self.free_counts.increment(self.n);
	}

	// According to the paper, the timestep can be updated after either every operation or only allocations
	pub fn inc_timer(&mut self) {
		self.n += 1;
		self.alloc_counts.add(self.n, self.alloc_counts.get(&(self.n-1)));
		self.alloc_sum.add(self.n, self.alloc_sum.get(&(self.n-1)));
		self.free_counts.add(self.n, self.free_counts.get(&(self.n-1)));
		self.free_sum.add(self.n, self.free_sum.get(&(self.n-1)));
	}

	// Evaluates liveness for windows of size k
	pub fn liveness(&self, k: usize) -> f32 {
		let i = self.n-k+1;
		let tmp1 = (self.m-self.free_counts.get(&i)) * i + self.free_sum.get(&i);
		let tmp2 = self.alloc_counts.get(&k) * k + self.alloc_sum.get(&self.n) - self.alloc_sum.get(&k);
		((tmp1 - tmp2 + self.m * k) as f32) / i as f32
	}
}

/*
	Reuse Counter
	Again, call alloc() and free() whenever needed
	To check if counter is currently in a burst, try sampling()
	inc_timer() works as described for liveness
	reuse(k) gets reuse for windows of length k
*/
pub struct ReuseCounter {
	burst_length: usize,		// Length of bursts
	hibernation_period: usize,	// Length of hibernation
	n: usize,					// Current time counter
	trace: Option<Trace>,		// Optional current trace -- none if hibernating	
	reuse: Option<HashMap<usize, f32>>	// Last calculated reuse -- none if not initialized (?)
}

impl ReuseCounter {
	pub fn new(bl: usize, hp: usize) -> ReuseCounter {
		ReuseCounter {
			burst_length: bl,
			hibernation_period: hp,
			n: 0,
			trace: Some(Trace::new()),	// Start sampling or hibernating?
			reuse: None
		}
	}

	pub fn alloc(&mut self, slot: usize) {
		match &mut self.trace {
			Some(t) => { t.add(Event::Alloc(slot)); }
			None => {}
		}
	}

	pub fn free(&mut self, slot: usize) {
		match &mut self.trace {
			Some(t) => { t.add(Event::Free(slot)); }
			None => {}
		}
	}

	// Maybe test if sampling before calling alloc and free? Not sure
	pub fn sampling(&self) -> bool {
		self.trace.is_some()
	}

	pub fn inc_timer(&mut self) -> () {
		self.n += 1;
		match &self.trace {
			Some(trace) => {
				if self.n >= self.burst_length {
					self.reuse = Some(reuse(trace));
					self.n = 0;
					self.trace = None;
				}
			}
			None => {
				if self.n >= self.hibernation_period {
					self.n = 0;
					self.trace = Some(Trace::new());
				}
			}
		}
	}

	pub fn reuse(&self, k: usize) -> Option<f32> {
		match &self.reuse {
			Some(reuse) => match reuse.get(&k) {
							Some(n) => Some(*n),
							None => Some(0.0)
						}
			None => None
		}
	}
}

// Offline Functions

fn reuse(t: &Trace) -> HashMap<usize, f32> {
	let intervals = t.free_intervals();
	let n = t.alloc_length();

	// Predicate terms
	let mut start_index_counts = vec![0; n];			// s_i
	let mut end_index_counts = vec![0; n];				// e_i
	let mut len_counts = vec![0; n];					// e_i - s_i -- indices decremented by 1 since no len 0
	let mut start_indices_sums = vec![0; n];			// I(e_i - s_i = k) * s_i -- indices decremented by 1
	let mut start_indices_min_sums = vec![0; n];		// I(e_i - s_i = k) * min(n-k, s_i) -- indices decremented by 1
	let mut end_indices_sums = vec![0; n];				// I(e_i - s_i = k) * e_i -- indices decremented by 1
	let mut end_indices_max_sums = vec![0; n];			// I(e_i - s_i = k) * max(k, e_i) -- indices decremented by 1

	for i in 0..intervals.len() {
		let interval = intervals[i];
		let len = interval.1 - interval.0 + 1;

		start_index_counts[interval.0] += 1;
		end_index_counts[interval.1] += 1;
		len_counts[len-1] += 1;
		start_indices_sums[len-1] += interval.0;
		start_indices_min_sums[len-1] += min(n-len, interval.0);
		end_indices_sums[len-1] += interval.1;
		end_indices_max_sums[len-1] += max(len, interval.1);
	}

	let mut start_index_n_k = vec![0; n];	// I(s_i >= (n-k))
	let mut end_index_k_1 = vec![0; n];		// I(e_i <= k-1)
	let mut len_l_k = vec![0; n];			// I(e_i - s_i <= k)

	start_index_n_k[0] = 0;	// Cannot start at index n -- remember index 0 -> k = 1
	end_index_k_1[0] = 0; // Cannot end at index 0
	len_l_k[0] = len_counts[0];	// I(e_i - s_i <= 1) = I(e_i - s_i = 1)

	for i in 1..n {
		start_index_n_k[i] = start_index_n_k[i-1] + start_index_counts[n-i];
		end_index_k_1[i] = end_index_k_1[i-1] + end_index_counts[i];
		len_l_k[i] = len_l_k[i-1] + len_counts[i];
	}
	let mut x = vec![0; n];	// X(i) = x[i-1]
	let mut y = vec![0; n];	// Y(i) = y[i-1]
	let mut z = vec![0; n];	// Z(i) = z[i-1]

	x[0] = start_indices_sums[0];
	y[0] = end_indices_sums[0];
	z[0] = len_counts[0];

	for i in 1..n {
		let k = i+1;

		x[i] = x[i-1] + start_indices_min_sums[i] - start_index_n_k[i];
		y[i] = y[i-1] + end_index_k_1[i-1] + end_indices_max_sums[i];
		z[i] = z[i-1] + len_l_k[i-1] + k * len_counts[i];
		println!("{}: {}, {}, {}", i, x[i], y[i], z[i]);
	}

	let mut result = HashMap::<usize, f32>::new();
	for k in 1..n+1 { result.insert(k, (x[k-1] + z[k-1] - y[k-1]) as f32 / (n-k+1) as f32 ); }

	result
}

#[cfg(test)]
mod test {
	use super::*;

	// Example from section 3.3
	#[test]
	fn test_liveness_counter() {
		let mut lc = LivenessCounter::new();
		lc.alloc();		// a1
		lc.inc_timer();
		lc.alloc();		// a2
		lc.inc_timer();
		lc.alloc();		// a3
		lc.inc_timer();
		lc.free();		// f1
		// lc.inc_timer();
		lc.free();		// f2
		// lc.inc_timer();
		lc.free();		// f3
		// lc.inc_timer();

		assert_eq!(lc.liveness(1), 2.0);
	}

	#[test]
	fn test_reuse_counter() {
		let mut rc = ReuseCounter::new(6, 18);
		rc.alloc(1); rc.inc_timer(); rc.alloc(2); rc.inc_timer(); rc.free(1); rc.alloc(1); rc.inc_timer(); rc.free(2); rc.alloc(2); rc.inc_timer();
		rc.free(1); rc.alloc(3); rc.inc_timer(); rc.alloc(1); rc.inc_timer();

		rc.free(1); rc.free(3); rc.alloc(3); rc.inc_timer();

		assert_eq!(rc.reuse(4), Some(7.0/3.0));
	}

	/* #[test]
	fn test_reuse_function() {
		let mut t = Trace::new();
		t.extend(vec![Event::Alloc(1), Event::Alloc(2), Event::Free(1), Event::Alloc(1), Event::Free(2), Event::Alloc(2), Event::Free(1), Event::Alloc(3), Event::Alloc(1)]);
		assert_eq!(*reuse(&t).get(&1).unwrap(), 2.0/6.0);
	}

	#[test]
	fn test_reuse_function_2() {
		let mut t = Trace::new();
		t.extend(vec![Event::Alloc(1), Event::Alloc(2), Event::Free(1), Event::Alloc(1), Event::Free(2), Event::Alloc(2), Event::Free(1), Event::Alloc(3), Event::Alloc(1)]);
		assert_eq!(*reuse(&t).get(&2).unwrap(), 1.0);
	}

	#[test]
	fn test_reuse_function_3() {
		let mut t = Trace::new();
		t.extend(vec![Event::Alloc(1), Event::Alloc(2), Event::Free(1), Event::Alloc(1), Event::Free(2), Event::Alloc(2), Event::Free(1), Event::Alloc(3), Event::Alloc(1)]);
		assert_eq!(*reuse(&t).get(&3).unwrap(), 7.0/4.0);
	}

	#[test]
	fn test_reuse_function_4() {
		let mut t = Trace::new();
		t.extend(vec![Event::Alloc(1), Event::Alloc(2), Event::Free(1), Event::Alloc(1), Event::Free(2), Event::Alloc(2), Event::Free(1), Event::Alloc(3), Event::Alloc(1)]);
		assert_eq!(*reuse(&t).get(&4).unwrap(), 7.0/3.0);
	}

	#[test]
	fn test_reuse_function_5() {
		let mut t = Trace::new();
		t.extend(vec![Event::Alloc(1), Event::Alloc(2), Event::Free(1), Event::Alloc(1), Event::Free(2), Event::Alloc(2), Event::Free(1), Event::Alloc(3), Event::Alloc(1)]);
		assert_eq!(*reuse(&t).get(&5).unwrap(), 5.0/2.0);
	}

	#[test]
	fn test_reuse_function_6() {
		let mut t = Trace::new();
		t.extend(vec![Event::Alloc(1), Event::Alloc(2), Event::Free(1), Event::Alloc(1), Event::Free(2), Event::Alloc(2), Event::Free(1), Event::Alloc(3), Event::Alloc(1)]);
		assert_eq!(*reuse(&t).get(&6).unwrap(), 3.0);
	} */
}