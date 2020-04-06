#[macro_export]
macro_rules! trace(
    ( $($thing:expr),* ) => { if cfg!(test) { println! { $($thing),* } } };
);
