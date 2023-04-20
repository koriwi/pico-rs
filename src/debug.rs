pub use defmt::debug as fmt_debug;
#[macro_export]
macro_rules! debug {
    ($($all:tt)*) => {
        #[cfg(feature = "debug")]
        fmt_debug!($($all)*)
    };
}
