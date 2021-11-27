#[cfg(not(target_os = "android"))]
mod arboard;
#[cfg(not(target_os = "android"))]
pub use self::arboard::*;

#[cfg(target_os = "android")]
mod termux;
#[cfg(target_os = "android")]
pub use termux::*;
