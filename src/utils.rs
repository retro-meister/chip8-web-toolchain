use std::fmt::Debug;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

pub fn vectors_equivalent<T: Eq + Debug>(vec1: Vec<T>, vec2: Vec<T>) -> bool {
    if vec1.len() != vec2.len() {
        println!("vecs not equal size! {} {}", vec1.len(), vec2.len());
        return false;
    }

    for (ai, bi) in vec1.iter().zip(vec2.iter()) {
        println!("{:?} {:?}", ai, bi);
        if ai != bi {
            return false;
        }
    }

    true
}

pub(crate) use log;
