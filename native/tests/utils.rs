use std::thread;

pub fn parallelize<F>(count: u32, f: F)
where
    F: Fn() + Sync + Send + Copy + 'static,
{
    let mut handles = Vec::new();
    for _ in 0..count {
        handles.push(thread::spawn(f));
    }
    for handle in handles {
        if handle.join().is_err() {
            panic!("thread panicked");
        }
    }
}
